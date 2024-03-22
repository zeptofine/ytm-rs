use iced::alignment::{self, Alignment};
use iced::widget::{
    button, checkbox, column, container, keyed_column, radio, row, scrollable, text, text_input,
    Column, Text,
};
use iced::{Command, Element, Font, Length, Subscription};

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Thumbnail {
    None,
    Filepath(PathBuf),
    // AtlasPiece()
}




