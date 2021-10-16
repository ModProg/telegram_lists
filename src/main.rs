use std::{
    convert::{TryFrom, TryInto},
    error::Error,
};

use teloxide::{
    payloads::{EditMessageReplyMarkupSetters, SendMessageSetters},
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardButtonKind, InlineKeyboardMarkup, ParseMode},
    utils::command::BotCommand,
};
use tokio_stream::wrappers::UnboundedReceiverStream;

use anyhow::*;

#[derive(BotCommand)]
#[command(rename = "lowercase", description = "These commands are supported:")]
enum Command {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "create a new list with a name.")]
    New(String),
}
fn make_keyboard() -> InlineKeyboardMarkup {
    let mut keyboard_array: Vec<Vec<InlineKeyboardButton>> = vec![];
    // The column is made by the list of Debian versions.
    let debian_versions = vec![
        "BuzzWordBecaouseINeedSomethingLonger",
        "Rex",
        "Bo",
        "Hamm",
        "Slink",
        "Potato",
        "Woody",
        "Sarge",
        "Etch",
        "Lenny",
        "Squeeze",
        "Wheezy",
        "Jessie",
        "Stretch",
        "Buster",
        "Bullseye",
    ];

    for version in debian_versions {
        // Match each button with the chat id and the Debian version.
        keyboard_array.push(vec![
            InlineKeyboardButton::callback(version.into(), format!("e_{}", version)),
            // InlineKeyboardButton::callback("➕".into(), format!("{}_{}", chat_id, version)),
            // InlineKeyboardButton::callback("➖".into(), format!("{}_{}", chat_id, version)),
            InlineKeyboardButton::callback("\u{2716}".into(), format!("d_{}", version)),
        ]);
    }

    InlineKeyboardMarkup::new(keyboard_array)
}

enum ButtonMode {
    Edit,
    Delete,
}

impl TryFrom<char> for ButtonMode {
    type Error = anyhow::Error;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        use ButtonMode::*;
        Ok(match value {
            'e' => Edit,
            'd' => Delete,
            _ => bail!("Invalid edit mode {}", value),
        })
    }
}

/// When it receives a callback from a button it edits the message with all
/// those buttons writing a text with the selected Debian version.
async fn callback_handler(cx: UpdateWithCx<AutoSend<Bot>, CallbackQuery>) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let (Some(data), Some(message)) = (&cx.update.data, &cx.update.message) {
        let mode: ButtonMode = data.chars().next().expect("There is always at least the mode in here").try_into()?;
        let id = data.get(2..).expect("There is always the id behind the mode");
        let items = message
            .reply_markup()
            .expect("Method can only be called on messages with reply_markup")
            .inline_keyboard
            .clone();
        let items = match mode {
            ButtonMode::Edit => todo!(),
            ButtonMode::Delete => items
                .into_iter()
                .filter(|row| match &row[0].kind {
                    InlineKeyboardButtonKind::CallbackData(s) => s.get(2..) != Some(id),
                    _ => unreachable!("Only callback buttons are used"),
                })
                .collect(),
        };
        let message_id = message.id;
        let chat_id = message.chat_id();

        if let Err(e) = cx
            .requester
            .edit_message_reply_markup(chat_id.to_string(), message_id)
            .reply_markup(InlineKeyboardMarkup { inline_keyboard: items })
            .send()
            .await
        {
            log::error!("{}", e);
        }
    }

    Ok(())
}

/// Parse the text wrote on Telegram and check if that text is a valid command
/// or not, then match the command. If the command is `/start` it writes a
/// markup with the `InlineKeyboardMarkup`.
async fn message_handler(cx: UpdateWithCx<AutoSend<Bot>, Message>) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Ok(command) = BotCommand::parse(cx.update.text().expect("Error with the text"), "buttons") {
        match command {
            Command::Help => {
                // Just send the description of all commands.
                cx.answer(Command::descriptions()).await?;
            }
            Command::New(mut name) => {
                if name.is_empty() {
                    name = "List".to_string()
                }
                cx.answer(format!("*{}*", name))
                    .parse_mode(ParseMode::MarkdownV2)
                    .reply_markup(make_keyboard())
                    .await?;
            }
        }
    } else {
        cx.reply_to("Command not found!").await?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    teloxide::enable_logging!();
    log::info!("Starting dices_bot...");

    let bot = Bot::from_env().auto_send();
    Dispatcher::new(bot)
        .messages_handler(|rx: DispatcherHandlerRx<AutoSend<Bot>, Message>| {
            UnboundedReceiverStream::new(rx).for_each_concurrent(None, |cx| async move {
                message_handler(cx).await.log_on_error().await;
            })
        })
        .callback_queries_handler(|rx: DispatcherHandlerRx<AutoSend<Bot>, CallbackQuery>| {
            UnboundedReceiverStream::new(rx).for_each_concurrent(None, |cx| async move {
                callback_handler(cx).await.log_on_error().await;
            })
        })
        .dispatch()
        .await;

    log::info!("Closing bot... Goodbye!");

    Ok(())
}
