
use crate::*;

pub const STATUS_BAR_HEIGHT: f32 = 60.0;

pub const BORDER_COLOR_ACTIVE: Color =         Color::srgb(0.75, 0.52, 0.99);
pub const BORDER_COLOR_INACTIVE: Color =       Color::srgb(0.25, 0.25, 0.25);
pub const TEXT_COLOR: Color =                  Color::srgb(0.9, 0.9, 0.9);
pub const BACKGROUND_COLOR: Color =            Color::srgb(0.15, 0.15, 0.15);
//pub const LLM_OUTPUT_BACKGROUND_COLOR: Color = Color::srgb(0.18, 0.12, 0.18); // 138,65,138
pub const LLM_OUTPUT_BACKGROUND_COLOR: Color = Color::srgba(0.18, 0.12, 0.18, 0.60); // 138,65,138

pub fn gui_setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
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
            //BackgroundColor(BACKGROUND_COLOR.into()),
            // Make this container node bundle to be Interactive so that clicking on it removes
            // focus from the text input.
            Interaction::None,
            GlobalZIndex(0),
        ))
        .with_children(|root_parent| {

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
                    ..default()
                },
                // Prevent clicks on the input from also bubbling down to the container
                // behind it
                bevy::ui::FocusPolicy::Block,
                BorderColor(BORDER_COLOR_INACTIVE.into()),
                BackgroundColor(BACKGROUND_COLOR.into()),
                bevy_simple_text_input::TextInput,
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
                    ..default()
                },
            )).with_children(|node_bundle| {
                node_bundle.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        top: Val::Px(0.0),
                        right: Val::Px(0.0),
                        bottom: Val::Px(0.0),
                        //width: Val::Px(400.0),
                        margin: UiRect::all(Val::Px(0.0)),
                        //border: UiRect::all(Val::Px(5.0)),
                        padding: UiRect::all(Val::Px(3.0)),
                        ..default()
                    },
                    Text::new("127.0.0.1:9050"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(TEXT_COLOR.into()),
                    BackgroundColor(BACKGROUND_COLOR.into()),
                    TextLayout::new_with_justify(JustifyText::Left),
                    gui_structs::Server_URL,
                ));
            });

        });

    commands.spawn(( // LLM reply text UI
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(4.0),
            left: Val::Px(4.0),
            right: Val::Px(4.0),
            bottom: Val::Px(STATUS_BAR_HEIGHT + 56.0),
            align_items: AlignItems::Start, // Start here means "Top"
            justify_content: JustifyContent::Start, // Start here means "Left"
            padding: UiRect::all(Val::Px(4.0)),
            ..default()
        },
        BorderColor(BORDER_COLOR_INACTIVE.into()),
        // BackgroundColor(LLM_OUTPUT_BACKGROUND_COLOR.into()),
        BackgroundColor::DEFAULT, // transparent!
        GlobalZIndex(100), // Text is above most things in this area
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
            BackgroundColor::DEFAULT, // transparent!
            gui_structs::LLM_ReplyText,
            ScrollableContent::default(),
        ));
    });

    commands.spawn(( // AI Image holder BEHIND the LLM text area
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(0.0),
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            bottom: Val::Px(STATUS_BAR_HEIGHT + 56.0),
            align_items: AlignItems::Start, // Start here means "Top"
            justify_content: JustifyContent::Start, // Start here means "Left"
            padding: UiRect::all(Val::Px(0.0)),
            ..default()
        },
        //BorderColor(BORDER_COLOR_INACTIVE.into()),
        //BackgroundColor(LLM_OUTPUT_BACKGROUND_COLOR.into()),
    ))
    .with_children(|image_area| {

        image_area.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(4.0),
                top: Val::Px(4.0),
                right: Val::Px(4.0),
                bottom: Val::Px(4.0),
                padding: UiRect::all(Val::Px(2.0)),
                margin: UiRect {
                    left: Val::Px(2.0),
                    top: Val::Px(2.0),
                    right: Val::Px(2.0),
                    bottom: Val::Px(6.0),
                },
                ..default()
            },
            BackgroundColor(LLM_OUTPUT_BACKGROUND_COLOR.into()),
            ZIndex(90),
        ));

        image_area.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                right: Val::Px(0.0),
                bottom: Val::Px(0.0),
                padding: UiRect::all(Val::Px(0.0)),
                margin: UiRect {
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    right: Val::Px(0.0),
                    //bottom: Val::Px(STATUS_BAR_HEIGHT + 2.0),
                    bottom: Val::Px(0.0),
                },
                //align_items: AlignItems::Center, // Start here means "Top"
                //justify_content: JustifyContent::Center, // Start here means "Left"
                ..default()
            },
            //Sprite::from_image(Image::transparent()),
            //Sprite::default()
            Sprite {
                image: asset_server.load("../../docs/img/ai-cliffs-02.png"),
                anchor: bevy::sprite::Anchor::Center,
                image_mode: bevy::ui::prelude::SpriteImageMode::Auto,
                custom_size: Some(bevy::math::f32::Vec2{x: 240.0, y:240.0}),
                ..default()
            },
            gui_structs::Background_Image,
            ZIndex(100),
        ));
    });


}
