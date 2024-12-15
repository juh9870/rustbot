use poise::serenity_prelude::{CreateActionRow, CreateButton, EditMessage};

pub trait IntoEdit {
    fn into_edit(self) -> EditMessage;
}

impl IntoEdit for CreateActionRow {
    fn into_edit(self) -> EditMessage {
        EditMessage::new().components(vec![self])
    }
}

impl IntoEdit for Vec<CreateActionRow> {
    fn into_edit(self) -> EditMessage {
        EditMessage::new().components(self)
    }
}

impl IntoEdit for CreateButton {
    fn into_edit(self) -> EditMessage {
        CreateActionRow::Buttons(vec![self]).into_edit()
    }
}

impl IntoEdit for Vec<CreateButton> {
    fn into_edit(self) -> EditMessage {
        CreateActionRow::Buttons(self).into_edit()
    }
}

impl IntoEdit for String {
    fn into_edit(self) -> EditMessage {
        EditMessage::new().content(self)
    }
}

impl IntoEdit for &str {
    fn into_edit(self) -> EditMessage {
        EditMessage::new().content(self)
    }
}
