use iced::widget::shader::{self, Pipeline, Primitive};
use oneiroi_wgpu::State;

pub struct OneiroiScene(State);

impl OneiroiScene {
    pub fn new() -> Self {
        //State::new(display, window)
        todo!()
    }
}

#[derive(Debug)]
pub struct Prim;

impl Primitive for Prim {
    type Pipeline = OneiroiPipe;

    fn prepare(
        &self,
        pipeline: &mut Self::Pipeline,
        device: &iced::wgpu::Device,
        queue: &iced::wgpu::Queue,
        bounds: &iced::Rectangle,
        viewport: &shader::Viewport,
    ) {
        todo!()
    }
}

impl<Message> shader::Program<Message> for OneiroiScene {
    type State = ();

    type Primitive = Prim;

    fn draw(
        &self,
        state: &Self::State,
        cursor: iced_core::mouse::Cursor,
        bounds: iced::Rectangle,
    ) -> Self::Primitive {
        todo!()
    }
}

pub struct OneiroiPipe;

impl Pipeline for OneiroiPipe {
    fn new(
        device: &iced::wgpu::Device,
        queue: &iced::wgpu::Queue,
        format: iced::wgpu::TextureFormat,
    ) -> Self
    where
        Self: Sized,
    {
        todo!()
    }
}
