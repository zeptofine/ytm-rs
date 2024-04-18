use iced::{
    advanced::widget::Id as WId,
    alignment::Vertical,
    widget::{button, column, container, pick_list, row, text, text_input, Column, Row, Space},
    Command as Cm, Element, Length, Renderer, Theme,
};
use serde::{Deserialize, Serialize};

use crate::{
    settings::{SongID, SongMap},
    song::ClosableSongMessage,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConstructorItem {
    Song(SongID),
    Operation(SongOpConstructor),
}

#[derive(Debug)]
pub enum PushErr {}

impl ConstructorItem {
    // TODO: These methods are ass

    pub fn push_to_id(
        &mut self,
        id: &WId,
        song_map: &SongMap,
        item: ConstructorItem,
    ) -> Result<(), PushErr> {
        println!["{:?}", self];
        match self {
            ConstructorItem::Song(s) => todo!(),
            ConstructorItem::Operation(op) => {
                if WId::from(op.id.0.clone()) == *id {
                    op.push(item)
                } else {
                    for (idx, child) in op.list.iter_mut().enumerate() {
                        println!["Child: {:#?}", child];
                        if child.item_has_id(song_map, id) {
                            // If the child is a song, put the new song before it
                            if let ConstructorItem::Song(_) = child {
                                op.insert(idx, item);
                            } else {
                                child.push_to_id(id, song_map, item)?;
                            }

                            break;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn item_has_id(&mut self, song_map: &SongMap, id: &WId) -> bool {
        match self {
            ConstructorItem::Song(ref key) if WId::from(song_map[key].id.0.clone()) == *id => true,
            ConstructorItem::Operation(op) => {
                if WId::from(op.id.0.clone()) == *id {
                    true
                } else {
                    op.list
                        .iter_mut()
                        .map(|i| i.item_has_id(song_map, id))
                        .count()
                        > 0
                }
            }
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum CItemMessage {
    Song(ClosableSongMessage),
    Operation(Box<SongOpMessage>),
}

#[derive(Debug, Clone)]
pub enum SongOpMessage {
    ChangeOperation(ActualRecursiveOps),
    CloseSelf,
    Add(ConstructorItem),
    Remove(usize),
    NewGroup,

    ItemMessage(usize, CItemMessage),
    Generate,

    Collapse,
    Uncollapse,

    ChangeN(u32),

    Null,
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
pub struct SongOpId(pub container::Id);
impl Default for SongOpId {
    fn default() -> Self {
        Self(container::Id::unique())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongOpConstructor {
    #[serde(skip)]
    id: SongOpId,
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
    fn header(
        &self,
        scheme: &FullYtmrsScheme,
        closable: bool,
    ) -> Row<'_, SongOpMessage, Theme, Renderer> {
        let scrollable_style = scheme.scrollable_style.clone();
        let pick_style = scheme.pick_list_style.clone();
        let pick_menu_style = scheme.pick_menu_style.clone();
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
            ConstructorItem::Song(id) => song_map[id]
                .view_closable(&scheme.song_appearance)
                .map(move |msg| SongOpMessage::ItemMessage(idx, CItemMessage::Song(msg))),
            ConstructorItem::Operation(constructor) => {
                constructor.view_nested(song_map, scheme).map(move |msg| {
                    SongOpMessage::ItemMessage(idx, CItemMessage::Operation(Box::new(msg)))
                })
            }
        });

        row![
            Space::with_width(Length::Fixed(2.0)),
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

    pub fn update(&mut self, msg: SongOpMessage) -> Cm<SongOpMessage> {
        match msg {
            SongOpMessage::CloseSelf => Cm::none(),
            SongOpMessage::Add(item) => {
                self.list.push(item);
                Cm::none()
            }
            SongOpMessage::Remove(idx) => {
                self.list.remove(idx);
                Cm::none()
            }
            SongOpMessage::ItemMessage(idx, msg) => {
                let item = &mut self.list[idx];
                match item {
                    ConstructorItem::Song(_id) => match msg {
                        CItemMessage::Song(msg) => match msg {
                            ClosableSongMessage::Closed => {
                                self.list.remove(idx);
                                Cm::none()
                            }
                            ClosableSongMessage::Clicked => todo!(),
                        },
                        CItemMessage::Operation(_) => todo!(), // Uh oh!!! This should be impossible!!!
                    },
                    ConstructorItem::Operation(op) => match msg {
                        CItemMessage::Song(_) => todo!(), // Uh oh!!! This should ALSO be impossible!!!
                        CItemMessage::Operation(somsg) => match *somsg {
                            SongOpMessage::CloseSelf => {
                                self.list.remove(idx);
                                Cm::none()
                            }
                            _ => op.update(*somsg).map(move |msg| {
                                SongOpMessage::ItemMessage(
                                    idx,
                                    CItemMessage::Operation(Box::new(msg)),
                                )
                            }),
                        },
                    },
                }
            }
            SongOpMessage::NewGroup => {
                self.list
                    .push(ConstructorItem::Operation(SongOpConstructor::default()));
                println!["Group added"];
                println!["{:#?}", self.list];
                Cm::none()
            }
            SongOpMessage::ChangeOperation(op) => {
                self.operation = op;
                Cm::none()
            }
            SongOpMessage::Collapse => {
                self.collapsed = true;
                Cm::none()
            }
            SongOpMessage::Uncollapse => {
                self.collapsed = false;
                Cm::none()
            }
            SongOpMessage::ChangeN(n) => {
                self.n = n;
                Cm::none()
            }

            SongOpMessage::Generate => {
                let ops = self.build();
                println!["{:#?}", ops];
                Cm::none()
            }

            // Pointer for things like inputting a non-integer value into the "N" field.
            SongOpMessage::Null => Cm::none(),
        }
    }

    pub fn build(&self) -> RecursiveSongOp {
        let children: Vec<RecursiveSongOp> = self
            .list
            .iter()
            .map(|item| match item {
                ConstructorItem::Song(id) => RecursiveSongOp::SinglePlay(id.clone()),
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
