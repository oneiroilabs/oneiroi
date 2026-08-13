use iced::window;

pub mod viewport;

#[derive(Debug, Clone)]
pub enum Message {
    OpenWindow,
    WindowOpened(window::Id),
    WindowClosed(window::Id),
    ScaleInputChanged(window::Id, String),
    ScaleChanged(window::Id, String),
    TitleChanged(window::Id, String),
    TabSelected(u32),
    SphereHit(window::Id),
}
