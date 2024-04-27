use std::collections::VecDeque;

use iced::{
    advanced::widget::Id as WId,
    alignment::Vertical,
    widget::{button, column, container, pick_list, row, text, text_input, Column, Row, Space},
    Command as Cm, Element, Length, Renderer, Theme,
};
use iced_drop::{droppable, zones_on_point};
use serde::{Deserialize, Serialize};

use crate::{
    settings::{SongKey, SongMap},
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
        match self {
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

#[derive(Debug)]
pub enum PushErr {
    NotFound,
}

pub trait TreeDirected {
    // TODO: These methods are not as ass but they could be better probably

    fn push_to_path(&mut self, pth: VecDeque<usize>, item: ConstructorItem);

    fn pop_path(&mut self, pth: VecDeque<usize>) -> Option<ConstructorItem>;

    fn item_has_id(&mut self, id: &WId) -> bool {
        self.path_to_id(id).is_some()
    }

    fn path_to_id(&self, id: &WId) -> Option<Vec<usize>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConstructorItem {
    Song(SongKey, #[serde(skip)] ItemId),
    Operation(SongOpConstructor),
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
    fn push_to_path(&mut self, pth: VecDeque<usize>, item: ConstructorItem) {
        match self {
            ConstructorItem::Song(_, _) => (),
            ConstructorItem::Operation(op) => op.push_to_path(pth, item),
        }
    }

    fn pop_path(&mut self, pth: VecDeque<usize>) -> Option<ConstructorItem> {
        match self {
            ConstructorItem::Song(_, _) => None,
            ConstructorItem::Operation(op) => op.pop_path(pth),
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
}

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

#[derive(Debug, Clone)]
pub enum CItemMessage {
    Operation(Box<SongOpMessage>),
}

#[derive(Debug, Clone)]
pub enum SongOpMessage {
    // User input
    NewGroup,
    Generate,
    ChangeN(u32),
    Collapse,
    Uncollapse,
    ChangeOperation(ActualRecursiveOps),
    CloseSelf,

    Add(ConstructorItem),
    Remove(usize),

    // Drag-n-drop
    Dropped(WId, iced::Point, iced::Rectangle),
    HandleZones(WId, Vec<(iced::advanced::widget::Id, iced::Rectangle)>),

    ItemMessage(usize, CItemMessage),

    Null,
}

pub enum UpdateResult {
    Cm(Cm<SongOpMessage>),
    SongClicked(WId),
    Move(WId, WId), // from, to
    None,
}

// #[derive(Debug, Clone, Default)]
// pub enum SelectionMode {
//     #[default]
//     None,
//     Single(usize),
//     Multiple(Vec<usize>),
//     Range(Range<usize>),
// }

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
    operation: ActualRecursiveOps,
    list: Vec<ConstructorItem>,
    collapsible: bool,
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
            collapsible: true,
            collapsed: false,
            n: 1,
        }
    }
}
impl SongOpConstructor {
    pub fn new(operation: ActualRecursiveOps, list: Vec<ConstructorItem>) -> Self {
        Self {
            id: ItemId::default(),
            operation,
            list,
            collapsible: true,
            collapsed: false,
            n: 1,
        }
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
            .push(button("Generate").on_press(SongOpMessage::Generate))
            .push(button("+").on_press(SongOpMessage::NewGroup))
            .into(),

            // Show a basic view of data
            true => row![
                text(format!(
                    "  {} - {} songs",
                    self.operation.as_str(),
                    self.list.len()
                ))
                .vertical_alignment(Vertical::Center),
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
            .align_items(iced::Alignment::Center)
    }

    fn get_children<'a>(
        &'a self,
        song_map: &'a SongMap,
        scheme: &FullYtmrsScheme,
    ) -> Row<'_, SongOpMessage, Theme, Renderer> {
        let items = self.list.iter().enumerate().map(|(idx, item)| match item {
            ConstructorItem::Song(key, sid) => {
                let song = &song_map[key];
                let img: Element<SongOpMessage> = song.get_img(75, 75);

                container(
                    row![
                        button(img).padding(0),
                        droppable(song.get_data())
                            .drag_mode(false, true)
                            .drag_hide(true)
                            .on_drop(move |pt, rec| SongOpMessage::Dropped(
                                WId::from(sid.0.clone()),
                                pt,
                                rec
                            )),
                        text(format!("{:?}", sid.0)),
                        button("x").on_press(SongOpMessage::Remove(idx))
                    ]
                    .align_items(iced::Alignment::Center),
                )
                .id(sid.0.clone())
                .into()
            }
            ConstructorItem::Operation(constructor) => {
                constructor.view_nested(song_map, scheme).map(move |msg| {
                    SongOpMessage::ItemMessage(idx, CItemMessage::Operation(Box::new(msg)))
                })
            }
        });

        row![
            Space::with_width(Length::Fixed(8.0)),
            Column::with_children(items)
        ]
        .width(Length::Fill)
    }

    pub fn view<'a>(
        &'a self,
        song_map: &'a SongMap,
        scheme: &FullYtmrsScheme,
    ) -> Element<SongOpMessage> {
        container(
            column![self.header(scheme, false).width(Length::Fill)]
                .push_maybe(match self.collapsed {
                    true => None,
                    false => Some(self.get_children(song_map, scheme)),
                })
                .width(Length::Fill),
        )
        .id(self.id.0.clone())
        .into()
    }

    pub fn view_nested<'a>(
        &'a self,
        song_map: &'a SongMap,
        scheme: &FullYtmrsScheme,
    ) -> Element<SongOpMessage> {
        container(
            column![self.header(scheme, true).width(Length::Fill)]
                .push_maybe(match self.collapsed {
                    true => None,
                    false => Some(self.get_children(song_map, scheme)),
                })
                .width(Length::Fill),
        )
        .id(self.id.0.clone())
        .into()
    }

    pub fn push(&mut self, item: ConstructorItem) {
        self.list.push(item);
    }

    pub fn insert(&mut self, idx: usize, item: ConstructorItem) {
        self.list.insert(idx, item)
    }

    pub fn update(&mut self, msg: SongOpMessage) -> UpdateResult {
        match msg {
            SongOpMessage::CloseSelf => UpdateResult::Cm(Cm::none()),
            SongOpMessage::Add(item) => {
                self.list.push(item);
                UpdateResult::None
            }
            SongOpMessage::Remove(idx) => {
                self.list.remove(idx);
                UpdateResult::None
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
                                UpdateResult::None
                            }
                            _ => match op.update(*somsg) {
                                UpdateResult::Cm(cm) => UpdateResult::Cm(cm.map(move |msg| {
                                    SongOpMessage::ItemMessage(
                                        idx,
                                        CItemMessage::Operation(Box::new(msg)),
                                    )
                                })),
                                UpdateResult::SongClicked(id) => UpdateResult::SongClicked(id),
                                UpdateResult::Move(from, to) => UpdateResult::Move(from, to),
                                UpdateResult::None => UpdateResult::None,
                            },
                        },
                    },
                }
            }
            SongOpMessage::NewGroup => {
                self.list
                    .push(ConstructorItem::Operation(SongOpConstructor::default()));
                println!["Group added"];
                println!["{:#?}", self.list];
                UpdateResult::None
            }
            SongOpMessage::ChangeOperation(op) => {
                self.operation = op;
                UpdateResult::None
            }
            SongOpMessage::Dropped(original_id, point, _rec) => UpdateResult::Cm(zones_on_point(
                move |zones| SongOpMessage::HandleZones(original_id.clone(), zones),
                point,
                None,
                None,
            )),
            SongOpMessage::HandleZones(original_id, zones) => {
                // TODO: This assumes the last zone was the desired target
                match zones.last() {
                    Some((target_id, _rec)) => match original_id == *target_id {
                        true => UpdateResult::SongClicked(original_id),
                        false => UpdateResult::Move(original_id, target_id.clone()),
                    },
                    None => UpdateResult::None,
                }
            }
            SongOpMessage::Collapse => {
                self.collapsed = true;
                UpdateResult::None
            }
            SongOpMessage::Uncollapse => {
                self.collapsed = false;
                UpdateResult::None
            }
            SongOpMessage::ChangeN(n) => {
                self.n = n;
                UpdateResult::None
            }

            SongOpMessage::Generate => {
                let ops = self.build();
                println!["{:#?}", ops];
                UpdateResult::None
            }

            // Pointer for things like inputting a non-integer value into the "N" field.
            SongOpMessage::Null => UpdateResult::None,
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
        Self::new(ActualRecursiveOps::PlayOnce, value)
    }
}
impl TreeDirected for SongOpConstructor {
    fn push_to_path(&mut self, mut pth: VecDeque<usize>, item: ConstructorItem) {
        let next_idx = pth.pop_front();
        match next_idx {
            None => {
                self.list.push(item);
            }
            Some(next_idx) => {
                let list_len = self.list.len();
                let subitem = &mut self.list[next_idx.min(list_len - 1)];
                match subitem {
                    ConstructorItem::Song(_, _) => self.insert(next_idx, item),
                    ConstructorItem::Operation(_) => subitem.push_to_path(pth, item),
                }
            }
        }
    }

    fn pop_path(&mut self, mut pth: VecDeque<usize>) -> Option<ConstructorItem> {
        let next_idx = pth.pop_front()?;
        let subitem = &mut self.list[next_idx];
        match subitem {
            ConstructorItem::Song(_, _) => Some(self.list.remove(next_idx)),
            ConstructorItem::Operation(op) => {
                if pth.is_empty() {
                    Some(op.list.remove(next_idx))
                } else {
                    subitem.pop_path(pth)
                }
            }
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
}
