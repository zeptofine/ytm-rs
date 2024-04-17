use iced::{
    widget::{button, column, pick_list, row, text, text_input, Column, Row, Space},
    Command as Cm, Element, Length, Renderer, Theme,
};
use serde::{Deserialize, Serialize};

use crate::{
    settings::{SongID, SongMap},
    song::ClosableSongMessage,
    styling::FullYtmrsScheme,
};

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
    NewGroup,

    ItemMessage(usize, CItemMessage),

    Remove(usize),
    Collapse,
    Uncollapse,
    ChangeN(u32),
    Null,
}

fn verify_n(txt: String) -> SongOpMessage {
    txt.parse::<u32>()
        .map(SongOpMessage::ChangeN)
        .unwrap_or(SongOpMessage::Null)
}
// A wrapper made for recursive song operations
const CONSTRUCTOR_CHOICES: [&'static str; 7] = [
    "Play Once",
    "Loop N Times",
    "Stretch",
    "Infinite Loop",
    "Random Play",
    "Single Random",
    "Infinite Random",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongOpConstructor {
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
            operation: ActualRecursiveOps::PlayOnce,
            list: vec![],
            collapsible: true,
            collapsed: false,
            n: 1,
        }
    }
}

impl SongOpConstructor {
    fn header(&self, closable: bool) -> Row<'_, SongOpMessage, Theme, Renderer> {
        let child: Element<SongOpMessage> = match self.collapsed {
            // show the operation controls
            false => row![pick_list(
                CONSTRUCTOR_CHOICES,
                Some(self.operation.as_str()),
                |selection| {
                    let op = ActualRecursiveOps::from_str(selection).unwrap();
                    SongOpMessage::ChangeOperation(op)
                },
            ),]
            .push_maybe(match self.operation {
                ActualRecursiveOps::LoopNTimes | ActualRecursiveOps::Stretch => Some(
                    text_input("1", &(format!("{}", self.n)))
                        .on_input(verify_n)
                        .on_paste(verify_n),
                ),
                _ => None,
            })
            .push(button("+").on_press(SongOpMessage::NewGroup))
            .into(),

            // Show a basic view of data
            true => text(format!(
                "{} - {} songs",
                self.operation.as_str(),
                self.list.len()
            ))
            .into(),
        };

        row![]
            .push_maybe(match self.collapsible {
                true => match self.collapsed {
                    true => Some(button(">").on_press(SongOpMessage::Uncollapse)),
                    false => Some(button("v").on_press(SongOpMessage::Collapse)),
                },
                false => None,
            })
            .push(child)
            .push_maybe(match closable {
                false => None,
                true => Some(button("x").on_press(SongOpMessage::CloseSelf)),
            })
    }

    pub fn view<'a>(
        &'a self,
        song_map: &'a SongMap,
        scheme: &FullYtmrsScheme,
    ) -> Element<SongOpMessage> {
        column![self.header(false).width(Length::Fill)]
            .push_maybe(match self.collapsed {
                true => None,
                false => Some(self.get_children(song_map, scheme)),
            })
            .width(Length::Fill)
            .into()
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

    pub fn view_nested<'a>(
        &'a self,
        song_map: &'a SongMap,
        scheme: &FullYtmrsScheme,
    ) -> Element<SongOpMessage> {
        column![self.header(true).width(Length::Fill)]
            .push_maybe(match self.collapsed {
                true => None,
                false => Some(self.get_children(song_map, scheme)),
            })
            .width(Length::Fill)
            .into()
    }

    pub fn push(&mut self, item: ConstructorItem) {
        self.list.push(item)
    }

    pub fn update(&mut self, msg: SongOpMessage) -> Cm<SongOpMessage> {
        println!["{msg:#?}"];
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
                    ConstructorItem::Song(id) => match msg {
                        CItemMessage::Song(msg) => match msg {
                            ClosableSongMessage::Closed => {
                                self.list.remove(idx);
                                Cm::none()
                            }
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
            // Pointer for things like inputting a non-integer value into the "N" field.
            SongOpMessage::Null => Cm::none(),
        }
    }
}
