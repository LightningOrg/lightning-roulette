//! Pequeños ayudantes de presentación compartidos por los juegos.

use lightning::Client;
use lightning::prelude::*;
use twilight_model::channel::message::MessageFlags;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};

/// Azul de Discord (`blurple`).
pub const BLURPLE: u32 = 0x5865F2;
/// Verde para victorias.
pub const GREEN: u32 = 0x57F287;
/// Rojo para derrotas.
pub const RED: u32 = 0xED4245;
/// Dorado para el blackjack y el menú de ayuda.
pub const GOLD: u32 = 0xF1C40F;
/// Gris azulado para resultados neutros.
pub const SLATE: u32 = 0x2C3E50;

/// Respuesta de texto sin menciones.
pub fn notice(text: impl Into<String>) -> Reply {
    Reply::text(text).no_mentions()
}

/// Envía un mensaje efímero (solo lo ve quien pulsó el botón).
///
/// El framework responde a un botón actualizando el mensaje original, así que
/// para los avisos privados hablamos directamente con la API de Discord a
/// través del cliente HTTP que Lightning deja en `extras`.
pub async fn whisper(ctx: &Context, text: impl Into<String>) {
    let Some(http) = ctx.ext::<Client>() else {
        return;
    };
    let Some(interaction) = ctx.interaction.as_ref() else {
        return;
    };

    let response = InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(InteractionResponseData {
            content: Some(text.into()),
            flags: Some(MessageFlags::EPHEMERAL),
            ..Default::default()
        }),
    };

    let _ = http
        .interaction(interaction.application_id)
        .create_response(interaction.id, &interaction.token, &response)
        .await;
}
