
use crate::*;

pub const STATUS_BAR_HEIGHT: f32 = 60.0;

pub const BORDER_COLOR_ACTIVE: Color =         Color::srgb(0.75, 0.52, 0.99);
pub const BORDER_COLOR_INACTIVE: Color =       Color::srgb(0.25, 0.25, 0.25);
pub const TEXT_COLOR: Color =                  Color::srgb(0.9, 0.9, 0.9);
pub const BACKGROUND_COLOR: Color =            Color::srgb(0.15, 0.15, 0.15);
pub const LLM_OUTPUT_BACKGROUND_COLOR: Color = Color::srgb(0.18, 0.12, 0.18); // 138,65,138

pub fn gui_setup(mut commands: Commands) {
    commands.spawn(Camera2d::default());

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::End, // End here means "Bottom"
                justify_content: JustifyContent::Start, // Start here means "Left"
                padding: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            bevy::ui::BackgroundColor(BACKGROUND_COLOR.into()),
            // Make this container node bundle to be Interactive so that clicking on it removes
            // focus from the text input.
            Interaction::None,
        ))
        .with_children(|root_parent| {

            root_parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(2.0),
                    top: Val::Px(2.0),
                    right: Val::Px(2.0),
                    bottom: Val::Px(2.0),
                    //width: Val::Percent(100.0),
                    border: UiRect::all(Val::Px(2.0)),
                    padding: UiRect::all(Val::Px(2.0)),
                    margin: UiRect {
                        left: Val::Px(2.0),
                        top: Val::Px(2.0),
                        right: Val::Px(2.0),
                        bottom: Val::Px(STATUS_BAR_HEIGHT + 2.0),
                    },
                    ..default()
                },
                GlobalZIndex(-1000),
                //Sprite::from_image(Image::transparent()),
                Sprite::default()
            ));

            root_parent.spawn((
                Node {
                    width: Val::Percent(80.0),
                    border: UiRect::all(Val::Px(2.0)),
                    padding: UiRect::all(Val::Px(2.0)),
                    margin: UiRect {
                        left: Val::Px(2.0),
                        top: Val::Px(2.0),
                        right: Val::Px(2.0),
                        bottom: Val::Px(STATUS_BAR_HEIGHT + 2.0),
                    },
                    // background_color: BACKGROUND_COLOR.into(),
                    // Prevent clicks on the input from also bubbling down to the container
                    // behind it
                    ..default()
                },
                bevy::ui::FocusPolicy::Block,
                BorderColor(BORDER_COLOR_INACTIVE.into()),
                bevy_simple_text_input::TextInput,
                    /*.with_text_style(TextStyle {
                        font_size: 32.0,
                        color: TEXT_COLOR,
                        ..default()
                    })
                    .with_placeholder("Type a message...", None)
                    .with_inactive(true),
                    */
                bevy_simple_text_input::TextInputTextFont(bevy_text::TextFont{ font_size: 32.0, ..default()}),
                bevy_simple_text_input::TextInputPlaceholder { value: "Type a message...".into(), text_font: None, text_color: Some(TEXT_COLOR.into())},
                bevy_simple_text_input::TextInputInactive(false),
            ));

            root_parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(4.0),
                    right: Val::Px(4.0),
                    bottom: Val::Px(2.0),
                    height: Val::Px(STATUS_BAR_HEIGHT),
                    align_items: AlignItems::End, // Start here means "Top"
                    justify_content: JustifyContent::Start, // Start here means "Left"
                    // z_index: ZIndex::Global(1000), // This ensures the text floats _above_ the LLM response text.
                    ..default()
                },
            )).with_children(|node_bundle| {
                node_bundle.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(1.0),
                        top: Val::Px(1.0),
                        right: Val::Px(1.0),
                        bottom: Val::Px(1.0),
                        //width: Val::Px(400.0),
                        margin: UiRect::all(Val::Px(1.0)),
                        //border: UiRect::all(Val::Px(5.0)),
                        padding: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    Text::new("127.0.0.1:9050"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR.into()),
                    TextLayout::new_with_justify(JustifyText::Left),

                    /*
                        // Accepts a `String` or any type that converts into a `String`, such as `&str`
                        ,
                        TextStyle {
                            // This font is loaded and will be used instead of the default font.
                            // font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                            font_size: 14.0,
                            color: TEXT_COLOR,
                            ..default()
                        },
                    ) // Set the justification of the Text
                    .with_text_justify(JustifyText::Left)
                    // Set the style of the TextBundle itself.
                    .with_style(Style {
                        position_type: PositionType::Absolute,
                        left: Val::Px(1.0),
                        top: Val::Px(1.0),
                        right: Val::Px(1.0),
                        bottom: Val::Px(1.0),
                        //width: Val::Px(400.0),
                        margin: UiRect::all(Val::Px(1.0)),
                        //border: UiRect::all(Val::Px(5.0)),
                        padding: UiRect::all(Val::Px(1.0)),
                        ..default()
                    }),
                    */

                    gui_structs::Server_URL,
                ));
            });

        });

    commands.spawn(( // TODO move me upstairs?
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(4.0),
            left: Val::Px(4.0),
            right: Val::Px(4.0),
            bottom: Val::Px(STATUS_BAR_HEIGHT + 56.0),
            align_items: AlignItems::Start, // Start here means "Top"
            justify_content: JustifyContent::Start, // Start here means "Left"
            padding: UiRect::all(Val::Px(4.0)),
//            border_color: BORDER_COLOR_INACTIVE.into(),
            // background_color: LLM_OUTPUT_BACKGROUND_COLOR.into(),
            ..default()
        },
        BorderColor(BORDER_COLOR_INACTIVE.into()),
        ScrollView::default(),
    ))
    .with_children(|scroll_area| {
        scroll_area.spawn((
            Node {
                margin: UiRect::all(Val::Px(4.0)),
                padding: UiRect::all(Val::Px(4.0)),
                ..default()
            },
            Text::new("Hello\nOliana!"),
            TextLayout::new_with_justify(JustifyText::Left),
            TextFont {
                font_size: 28.0,
                ..default()
            },
            TextColor(TEXT_COLOR.into()),
            /*
                // Accepts a `String` or any type that converts into a `String`, such as `&str`
                "Hello\nOliana!",
                TextStyle {
                    // This font is loaded and will be used instead of the default font.
                    // font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 28.0,
                    ..default()
                },
            ) // Set the justification of the Text
            .with_text_justify(JustifyText::Left)
            // Set the style of the TextBundle itself.
            .with_style(Style {
                //align_self: AlignSelf::Stretch, // NB: This breaks the ScrollView behavior!
                margin: UiRect::all(Val::Px(4.0)),
                padding: UiRect::all(Val::Px(4.0)),
                ..default()
            }),
            */

            gui_structs::LLM_ReplyText,
            ScrollableContent::default(),
        ));
    });


}
