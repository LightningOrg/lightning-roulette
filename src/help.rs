//! Menú de ayuda del casino.

use lightning::prelude::*;

use crate::ui;

/// Muestra los minijuegos disponibles y cómo jugar
#[command(aliases = ["ayuda", "juegos"])]
async fn casino() -> Reply {
    Reply::new()
        .embed(
            Embed::new()
                .title("🎰 Casino")
                .description(
                    "Empiezas con **1.000 fichas**. Usa los comandos con `/`; si activas \
                     el intent `MESSAGE_CONTENT` y defines `DISCORD_PREFIX`, también \
                     funcionan con prefijo (por ejemplo `!blackjack 100`).",
                )
                .color(ui::GOLD)
                .field(
                    "🃏 Blackjack",
                    "`/blackjack apuesta:100`\n\
                     Pide cartas, plántate, dobla o retírate. El blackjack paga **3:2**.",
                    false,
                )
                .field(
                    "🎡 Ruleta",
                    "`/ruleta apuesta:100 tipo:rojo`\n\
                     Tipos: `rojo`, `negro`, `par`, `impar`, `bajo`, `alto` o `numero` \
                     (con el número del 0 al 36). Color y paridad pagan **2x**; un número \
                     paga **36x**.",
                    false,
                )
                .field(
                    "🎰 Tragaperras",
                    "`/slots apuesta:100`\n\
                     Tres iguales pagan de **4x a 50x**. Dos iguales devuelven **2x**.",
                    false,
                )
                .field(
                    "💰 Cartera",
                    "`/saldo`\n\
                     Consulta cuántas fichas te quedan.",
                    false,
                )
                .footer("Juega con cabeza: aquí las fichas no valen dinero de verdad."),
        )
        .no_mentions()
}
