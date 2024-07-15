use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use iced::{
    advanced::widget::Id as WId,
    alignment::Vertical,
    widget::{
        button, column, container, pick_list, row, text, text_input, Column, Container, Row, Space,
    },
    Element, Length, Renderer, Task as T, Theme,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::{
    caching::{BufferedCache, NDJsonCache},
    settings::SongKey,
    song::{Song, SongData},
    styling::FullYtmrsScheme,
};

use super::RecursiveSongOp;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActualRecursiveOps {
    PlayOnce,
    LoopNTimes,
    Stretch,
    InfiniteLoop,
    RandomPlay,
    SingleRandom,
    InfiniteRandom,
}
impl ActualRecursiveOps {
    fn as_str(&self) -> &'static str {
        match &self {
            ActualRecursiveOps::PlayOnce => CONSTRUCTOR_CHOICES[0],
            ActualRecursiveOps::LoopNTimes => CONSTRUCTOR_CHOICES[1],
            ActualRecursiveOps::Stretch => CONSTRUCTOR_CHOICES[2],
            ActualRecursiveOps::InfiniteLoop => CONSTRUCTOR_CHOICES[3],
            ActualRecursiveOps::RandomPlay => CONSTRUCTOR_CHOICES[4],
            ActualRecursiveOps::SingleRandom => CONSTRUCTOR_CHOICES[5],
            ActualRecursiveOps::InfiniteRandom => CONSTRUCTOR_CHOICES[6],
        }
    }

    fn from_str(s: &'static str) -> Option<ActualRecursiveOps> {
        match s {
            "Play Once" => Some(ActualRecursiveOps::PlayOnce),
            "Loop N Times" => Some(ActualRecursiveOps::LoopNTimes),
            "Stretch" => Some(ActualRecursiveOps::Stretch),
            "Infinite Loop" => Some(ActualRecursiveOps::InfiniteLoop),
            "Random Play" => Some(ActualRecursiveOps::RandomPlay),
            "Single Random" => Some(ActualRecursiveOps::SingleRandom),
            "Infinite Random" => Some(ActualRecursiveOps::InfiniteRandom),
            _ => None,
        }
    }
}

pub trait TreeDirected {
    // TODO: These methods are not as ass but they could be better probably

    fn push_to_path(&mut self, pth: &[usize], item: ConstructorItem) {
        self.push_to_path_(pth, 0, item)
    }

    fn push_to_path_(&mut self, pth: &[usize], pidx: usize, item: ConstructorItem);

    fn pop_path(&mut self, pth: &[usize]) -> Option<ConstructorItem> {
        self.pop_path_(pth, 0)
    }

    fn pop_path_(&mut self, pth: &[usize], pidx: usize) -> Option<ConstructorItem>;

    fn item_has_id(&mut self, id: &WId) -> bool {
        self.path_to_id(id).is_some()
    }

    fn path_to_id(&self, id: &WId) -> Option<Vec<usize>>;

    fn item_at_path(&self, pth: &[usize]) -> Option<&ConstructorItem> {
        self.item_at_path_(pth, 0)
    }

    fn item_at_path_(&self, pth: &[usize], idx: usize) -> Option<&ConstructorItem>;

    fn item_at_path_mut(&mut self, pth: &[usize]) -> Option<&mut ConstructorItem> {
        self.item_at_path_mut_(pth, 0)
    }

    fn item_at_path_mut_(&mut self, pth: &[usize], idx: usize) -> Option<&mut ConstructorItem>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConstructorItem {
    Song(SongKey, #[serde(skip)] ItemId),
    Operation(SongOpConstructor),
}
impl ConstructorItem {
    pub fn all_song_keys(&self) -> Vec<&SongKey> {
        match *self {
            ConstructorItem::Song(ref key, _) => vec![key],
            ConstructorItem::Operation(ref op) => op.all_song_keys_rec().collect(),
        }
    }
}
impl From<SongKey> for ConstructorItem {
    fn from(value: SongKey) -> Self {
        Self::Song(value, ItemId::default())
    }
}
impl From<SongOpConstructor> for ConstructorItem {
    fn from(value: SongOpConstructor) -> Self {
        Self::Operation(value)
    }
}

impl TreeDirected for ConstructorItem {
    fn push_to_path_(&mut self, pth: &[usize], pidx: usize, item: ConstructorItem) {
        match self {
            ConstructorItem::Song(_, _) => (),
            ConstructorItem::Operation(op) => op.push_to_path_(pth, pidx, item),
        }
    }

    fn pop_path_(&mut self, pth: &[usize], pidx: usize) -> Option<ConstructorItem> {
        match self {
            ConstructorItem::Song(_, _) => None,
            ConstructorItem::Operation(op) => op.pop_path_(pth, pidx),
        }
    }

    fn path_to_id(&self, id: &WId) -> Option<Vec<usize>> {
        match self {
            ConstructorItem::Song(_key, sid) => {
                let sid = WId::from(sid.0.clone());
                match sid == *id {
                    true => Some(vec![]),
                    false => None,
                }
            }
            ConstructorItem::Operation(op) => op.path_to_id(id),
        }
    }

    fn item_at_path_(&self, pth: &[usize], idx: usize) -> Option<&ConstructorItem> {
        if pth.len() == idx {
            return Some(self);
        }
        match self {
            ConstructorItem::Song(_, _) => None,
            ConstructorItem::Operation(op) => op.item_at_path_(pth, idx + 1),
        }
    }

    fn item_at_path_mut_(&mut self, pth: &[usize], idx: usize) -> Option<&mut ConstructorItem> {
        if pth.len() == idx {
            return Some(self);
        }
        match self {
            ConstructorItem::Song(_, _) => None,
            ConstructorItem::Operation(op) => op.item_at_path_mut_(pth, idx + 1),
        }
    }
}

#[cfg(test)]
mod tests {
    use iced::advanced::widget::Id as WId;

    use crate::song_operations::{ConstructorItem, ItemId, SongOpConstructor, TreeDirected};

    #[test]
    fn test_path_to_id() {
        let song_id = ItemId::default();
        let song = ConstructorItem::Song("hell".to_string(), song_id.clone());
        let song_id2 = ItemId::default();
        let song2 = ConstructorItem::Song("hell".to_string(), song_id2.clone());

        let subtree = SongOpConstructor::from(vec![song2]);
        let subtree_id = subtree.id.clone();

        let list = vec![song, ConstructorItem::Operation(subtree)];
        let tree = SongOpConstructor::from(list);

        let unused_id = ItemId::default();

        assert_eq![Some(vec![0]), tree.path_to_id(&WId::from(song_id.0))];
        assert_eq![Some(vec![1]), tree.path_to_id(&WId::from(subtree_id.0))];
        assert_eq![Some(vec![1, 0]), tree.path_to_id(&WId::from(song_id2.0))];
        assert_eq![None, tree.path_to_id(&WId::from(unused_id.0))];
    }
}

#[derive(Debug, Clone)]
pub enum CItemMessage {
    Operation(Box<SongOpMessage>),
}

#[derive(Debug, Clone)]
pub enum SongOpMessage {
    // User input
    NewGroup,
    ChangeN(u32),
    Collapse,
    Uncollapse,
    ChangeOperation(ActualRecursiveOps),
    CloseSelf,

    Remove(usize),

    // Drag-n-drop
    Dropped(WId, iced::Point, iced::Rectangle),
    HandleZones(WId, Vec<(iced::advanced::widget::Id, iced::Rectangle)>),
    SongClicked(WId),

    ItemMessage(usize, CItemMessage),

    Null,
}

pub enum UpdateResult {
    Cm(T<SongOpMessage>),
    SongClicked(WId),
    Move(WId, WId), // from, to
}

fn verify_n(txt: String) -> SongOpMessage {
    txt.parse::<u32>()
        .map(SongOpMessage::ChangeN)
        .unwrap_or(SongOpMessage::Null)
}
// A wrapper made for recursive song operations
const CONSTRUCTOR_CHOICES: [&str; 7] = [
    "Play Once",
    "Loop N Times",
    "Stretch",
    "Infinite Loop",
    "Random Play",
    "Single Random",
    "Infinite Random",
];

#[derive(Debug, Clone)]
pub struct ItemId(pub container::Id);
impl Default for ItemId {
    fn default() -> Self {
        Self(container::Id::unique())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongOpConstructor {
    #[serde(skip)]
    id: ItemId,
    pub operation: ActualRecursiveOps,
    pub list: Vec<ConstructorItem>,
    #[serde(skip)]
    cache: Option<Arc<RwLock<NDJsonCache<Song>>>>,
    pub collapsible: bool,
    collapsed: bool,
    // used for certain operations, like LoopNTimes and Stretch
    n: u32,
}
impl Default for SongOpConstructor {
    fn default() -> Self {
        Self {
            id: Default::default(),
            operation: ActualRecursiveOps::PlayOnce,
            list: vec![],
            cache: None,
            collapsible: true,
            collapsed: false,
            n: 1,
        }
    }
}
impl SongOpConstructor {
    pub fn new(
        operation: ActualRecursiveOps,
        list: Vec<ConstructorItem>,
        cache: Option<Arc<RwLock<NDJsonCache<Song>>>>,
    ) -> Self {
        Self {
            id: ItemId::default(),
            operation,
            list,
            cache,
            collapsible: true,
            collapsed: false,
            n: 1,
        }
    }

    pub fn set_cache(&mut self, cache: Arc<RwLock<NDJsonCache<Song>>>) {
        self.cache = Some(cache.clone());
        for item in &mut self.list {
            if let ConstructorItem::Operation(op) = item {
                op.set_cache(cache.clone());
            }
        }
    }

    /// Returns all the song keys found in this constructor recursively
    pub fn all_song_keys_rec(&self) -> impl Iterator<Item = &SongKey> {
        self.list.iter().flat_map(|item| item.all_song_keys())
    }

    fn header(
        &self,
        scheme: &FullYtmrsScheme,
        closable: bool,
    ) -> Row<'_, SongOpMessage, Theme, Renderer> {
        let pick_style = scheme.pick_list_style.clone();
        let child: Element<SongOpMessage> = match self.collapsed {
            // show the operation controls
            false => row![pick_list(
                CONSTRUCTOR_CHOICES,
                Some(self.operation.as_str()),
                |selection| {
                    let op = ActualRecursiveOps::from_str(selection).unwrap();
                    SongOpMessage::ChangeOperation(op)
                },
            )
            .style(pick_style.update()),]
            .push_maybe(match self.operation {
                ActualRecursiveOps::LoopNTimes | ActualRecursiveOps::Stretch => Some(
                    text_input("1", &(format!("{}", self.n)))
                        .on_input(verify_n)
                        .on_paste(verify_n),
                ),
                _ => None,
            })
            .push(Space::with_width(Length::Fill))
            .push(button("+").on_press(SongOpMessage::NewGroup))
            .into(),

            // Show a basic view of data
            true => row![
                text(format!(
                    "  {} - {} songs",
                    self.operation.as_str(),
                    self.list.len()
                ))
                .align_y(Vertical::Center),
                Space::with_width(Length::Fill)
            ]
            .into(),
        };

        row![]
            .push_maybe(match self.collapsible {
                true => match self.collapsed {
                    true => Some(button(">").on_press(SongOpMessage::Uncollapse).width(30)),
                    false => Some(button("v").on_press(SongOpMessage::Collapse).width(30)),
                },
                false => None,
            })
            .push(child)
            .push_maybe(match closable {
                false => None,
                true => Some(button("x").on_press(SongOpMessage::CloseSelf)),
            })
            .spacing(0)
            .align_y(Vertical::Center)
    }

    fn get_children(&self, scheme: &FullYtmrsScheme) -> Row<'_, SongOpMessage, Theme, Renderer> {
        let songs: HashSet<String> = self
            .list
            .iter()
            .filter_map(|item| match item {
                ConstructorItem::Song(key, _) => Some(key),
                ConstructorItem::Operation(_) => None,
            })
            .cloned()
            .collect();

        let map: HashMap<String, _> = match &self.cache {
            Some(lock) => {
                let c = lock.read();
                c.fetch_existing(&songs)
            }
            None => HashMap::new(),
        };

        let items = self.list.iter().enumerate().map(|(idx, item)| match item {
            ConstructorItem::Song(key, sid) => {
                let data = {
                    match map.get(key) {
                        Some(arc) => {
                            let x = arc.read();
                            x.as_data()
                        }
                        None => SongData::mystery_with_title(key.clone()),
                    }
                };
                let swid = WId::from(sid.0.clone());
                let song = Element::new(data.row(true, true))
                    .map(move |_| SongOpMessage::SongClicked(swid.clone()));

                container(
                    row![
                        song,
                        // droppable()
                        //     .drag_mode(false, true)
                        //     .drag_hide(true)
                        //     .on_single_click(SongOpMessage::SongClicked(wid.clone()))
                        //     .on_drop(move |pt, rec| SongOpMessage::Dropped(wid.clone(), pt, rec)),
                        button("x").on_press(SongOpMessage::Remove(idx))
                    ]
                    .align_y(iced::Alignment::Center),
                )
                .id(sid.0.clone())
                .into()
            }
            ConstructorItem::Operation(constructor) => Element::new(
                constructor.view_nested(scheme), // droppable()
                                                 //     .drag_mode(false, true)
                                                 //     .drag_hide(true)
                                                 //     .on_drag(move |_, _| SongOpMessage::Collapse)
                                                 //     .on_drop(move |pt, rec| {
                                                 //         SongOpMessage::Dropped(constructor.id.0.clone().into(), pt, rec)
                                                 //     }),
            )
            .map(move |msg| {
                SongOpMessage::ItemMessage(idx, CItemMessage::Operation(Box::new(msg)))
            }),
        });

        row![
            Space::with_width(Length::Fixed(8.0)),
            Column::with_children(items)
        ]
        .width(Length::Fill)
    }

    pub fn view(&self, scheme: &FullYtmrsScheme) -> Container<SongOpMessage> {
        container(
            column![self.header(scheme, false).width(Length::Fill)]
                .push_maybe(match self.collapsed {
                    true => None,
                    false => Some(self.get_children(scheme)),
                })
                .width(Length::Fill),
        )
        .id(self.id.0.clone())
    }

    pub fn view_nested(&self, scheme: &FullYtmrsScheme) -> Container<SongOpMessage> {
        container(
            column![self.header(scheme, true).width(Length::Fill)]
                .push_maybe(match self.collapsed {
                    true => None,
                    false => Some(self.get_children(scheme)),
                })
                .width(Length::Fill),
        )
        .id(self.id.0.clone())
    }

    pub fn insert(&mut self, idx: usize, item: ConstructorItem) {
        self.list.insert(idx, item)
    }

    pub fn update(&mut self, msg: SongOpMessage) -> Option<UpdateResult> {
        match msg {
            SongOpMessage::CloseSelf => None,
            SongOpMessage::Remove(idx) => {
                self.list.remove(idx);
                None
            }
            SongOpMessage::ItemMessage(idx, msg) => {
                let item = &mut self.list[idx];
                match item {
                    ConstructorItem::Song(_key, _sid) => match msg {
                        CItemMessage::Operation(_) => todo!(), // Uh oh!!! This should be impossible!!!
                    },
                    ConstructorItem::Operation(op) => match msg {
                        CItemMessage::Operation(somsg) => match *somsg {
                            SongOpMessage::CloseSelf => {
                                self.list.remove(idx);
                                None
                            }
                            _ => match op.update(*somsg) {
                                Some(result) => match result {
                                    UpdateResult::Cm(cm) => {
                                        Some(UpdateResult::Cm(cm.map(move |msg| {
                                            SongOpMessage::ItemMessage(
                                                idx,
                                                CItemMessage::Operation(Box::new(msg)),
                                            )
                                        })))
                                    }
                                    UpdateResult::SongClicked(id) => {
                                        Some(UpdateResult::SongClicked(id))
                                    }
                                    UpdateResult::Move(from, to) => {
                                        Some(UpdateResult::Move(from, to))
                                    }
                                },
                                None => None,
                            },
                        },
                    },
                }
            }
            SongOpMessage::NewGroup => {
                let constructor = SongOpConstructor {
                    cache: self.cache.clone(),
                    ..Default::default()
                };
                self.list.push(ConstructorItem::Operation(constructor));
                println!["Group added"];
                println!["{:#?}", self.list];
                None
            }
            SongOpMessage::ChangeOperation(op) => {
                self.operation = op;
                None
            }
            SongOpMessage::Dropped(_original_id, _point, _rec) => {
                // Some(UpdateResult::Cm(zones_on_point(
                //     move |zones| SongOpMessage::HandleZones(original_id.clone(), zones),
                //     point,
                //     None,
                //     None,
                // )))
                None
            }
            SongOpMessage::HandleZones(original_id, zones) => {
                // TODO: This assumes the last zone was the desired target
                match zones.last() {
                    Some((target_id, _rec)) => {
                        if original_id != *target_id {
                            Some(UpdateResult::Move(original_id, target_id.clone()))
                        } else {
                            None
                        }
                    }
                    None => None,
                }
            }
            SongOpMessage::Collapse => {
                self.collapsed = true;
                None
            }
            SongOpMessage::Uncollapse => {
                self.collapsed = false;
                None
            }
            SongOpMessage::ChangeN(n) => {
                self.n = n;
                None
            }

            // Pointer for things like inputting a non-integer value into the "N" field.
            SongOpMessage::Null => None,
            SongOpMessage::SongClicked(wid) => Some(UpdateResult::SongClicked(wid)),
        }
    }

    pub fn build(&self) -> RecursiveSongOp {
        let children: Vec<RecursiveSongOp> = self
            .list
            .iter()
            .map(|item| match item {
                ConstructorItem::Song(key, _) => RecursiveSongOp::SinglePlay(key.clone()),
                ConstructorItem::Operation(op) => op.build(),
            })
            .collect();

        match &self.operation {
            ActualRecursiveOps::PlayOnce => RecursiveSongOp::PlayOnce(children),
            ActualRecursiveOps::LoopNTimes => RecursiveSongOp::LoopNTimes(children, self.n),
            ActualRecursiveOps::Stretch => RecursiveSongOp::Stretch(children, self.n),
            ActualRecursiveOps::InfiniteLoop => RecursiveSongOp::InfiniteLoop(children),
            ActualRecursiveOps::RandomPlay => RecursiveSongOp::RandomPlay(children),
            ActualRecursiveOps::SingleRandom => RecursiveSongOp::SingleRandom(children),
            ActualRecursiveOps::InfiniteRandom => RecursiveSongOp::InfiniteRandom(children),
        }
    }
}
impl From<Vec<ConstructorItem>> for SongOpConstructor {
    fn from(value: Vec<ConstructorItem>) -> Self {
        Self::new(ActualRecursiveOps::PlayOnce, value, None)
    }
}
impl TreeDirected for SongOpConstructor {
    fn push_to_path_(&mut self, pth: &[usize], pidx: usize, item: ConstructorItem) {
        match pth.get(pidx) {
            None => {
                self.list.push(item);
            }
            Some(next_idx) => {
                let list_len = self.list.len();
                let subitem = &mut self.list[*next_idx.min(&(list_len - 1))];
                match subitem {
                    ConstructorItem::Song(_, _) => self.insert(*next_idx, item),
                    ConstructorItem::Operation(_) => {
                        subitem.push_to_path_(pth, pidx + 1, item);
                    }
                }
            }
        }
    }

    fn pop_path_(&mut self, pth: &[usize], pidx: usize) -> Option<ConstructorItem> {
        match pth.get(pidx) {
            None => None,
            Some(next_idx) => match self.list.get_mut(*next_idx) {
                Some(subitem) => match subitem {
                    ConstructorItem::Song(_, _) => Some(self.list.remove(*next_idx)),
                    ConstructorItem::Operation(_) => {
                        if pth.len() == pidx + 1 {
                            Some(self.list.remove(*next_idx))
                        } else {
                            subitem.pop_path_(pth, pidx + 1)
                        }
                    }
                },
                None => None,
            },
        }
    }

    fn path_to_id(&self, id: &WId) -> Option<Vec<usize>> {
        let oid = WId::from(self.id.0.clone());
        let mut v = vec![];
        match oid == *id {
            true => Some(v),
            false => match self
                .list
                .iter()
                .enumerate()
                .find_map(|(idx, item)| item.path_to_id(id).map(|v| (idx, v)))
            {
                Some((idx, ids)) => {
                    v.push(idx);
                    v.extend(ids);
                    Some(v)
                }
                None => None,
            },
        }
    }

    fn item_at_path_(&self, pth: &[usize], idx: usize) -> Option<&ConstructorItem> {
        if pth.len() == idx {
            return None;
        }
        let subitem = self.list.get(idx)?;
        if pth.len() == idx + 1 {
            Some(subitem)
        } else {
            subitem.item_at_path_(pth, idx + 1)
        }
    }

    fn item_at_path_mut_(&mut self, pth: &[usize], idx: usize) -> Option<&mut ConstructorItem> {
        if pth.len() == idx {
            return None;
        }
        let subitem = self.list.get_mut(idx)?;
        if pth.len() == idx + 1 {
            Some(subitem)
        } else {
            subitem.item_at_path_mut_(pth, idx + 1)
        }
    }
}
