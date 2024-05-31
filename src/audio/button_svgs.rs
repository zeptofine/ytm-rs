use iced::advanced;
use once_cell::sync::Lazy;

const PLAY_SVG_DATA: &[u8] =
    include_bytes!("../../assets/play_arrow_40dp_FILL0_wght400_GRAD0_opsz40.svg");

const PAUSE_SVG_DATA: &[u8] =
    include_bytes!("../../assets/pause_40dp_FILL0_wght400_GRAD0_opsz40.svg");

const SKIP_NEXT_SVG_DATA: &[u8] =
    include_bytes!("../../assets/skip_next_40dp_FILL0_wght400_GRAD0_opsz40.svg");

pub static PLAY_SVG: Lazy<advanced::svg::Handle> =
    Lazy::new(|| advanced::svg::Handle::from_memory(PLAY_SVG_DATA));

pub static PAUSE_SVG: Lazy<advanced::svg::Handle> =
    Lazy::new(|| advanced::svg::Handle::from_memory(PAUSE_SVG_DATA));

pub static SKIP_NEXT_SVG: Lazy<advanced::svg::Handle> =
    Lazy::new(|| advanced::svg::Handle::from_memory(SKIP_NEXT_SVG_DATA));
