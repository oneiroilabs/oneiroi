use iced::{
    Color, Length, Padding,
    alignment::Horizontal,
    widget::{Container, container, row, text},
};
use iced_nodegraph::{node_header, pin};

struct TypedSlot;

pub fn my_node<'a, Message>(theme: &'a iced::Theme) -> iced::Element<'a, Message>
where
    Message: Clone + 'a,
{
    //let style = NodeContentStyle::process(theme);

    let pin_list = iced::widget::column![
        // Row 1: email input + subject output
        row![
            container(pin!(
                Left,
                "email",
                text("email"),
                Input,
                ::std::any::TypeId::of::<TypedSlot>()
            ))
            .width(Length::FillPortion(1))
            .align_x(Horizontal::Left),
            container(pin!(
                Right,
                "subject",
                text("subject"),
                Output,
                ::std::any::TypeId::of::<TypedSlot>()
            ))
            .width(Length::FillPortion(1))
            .align_x(Horizontal::Right),
        ]
        .width(Length::Fill),
        // Row 2: datetime output
        container(pin!(
            Right,
            "datetime",
            text("datetime"),
            Output,
            ::std::any::TypeId::of::<TypedSlot>()
        ))
        .width(Length::Fill)
        .align_x(Horizontal::Right),
        // Row 3: body output
        container(pin!(
            Right,
            "body",
            text("body"),
            Output,
            ::std::any::TypeId::of::<TypedSlot>()
        ))
        .width(Length::Fill)
        .align_x(Horizontal::Right),
    ]
    .spacing(4);

    iced::widget::column![
        node_title_bar("Email Parser"),
        container(pin_list).padding([10, 12])
    ]
    .width(200.0)
    .into()
}

use iced::Theme;

pub fn node_title_bar<'a, Message>(
    title: impl Into<String>,
    //style: NodeContentStyle,
) -> Container<'a, Message, Theme, iced::Renderer>
where
    Message: Clone + 'a,
{
    let title_text = text(title.into()).size(13).color(Color::WHITE);

    node_header(
        container(title_text).padding(Padding {
            top: 4.0,
            bottom: 4.0,
            left: 8.0,
            right: 8.0,
        }),
        Color::BLACK,
        //style.title_background,
        1., //style.corner_radius,
    )
}
