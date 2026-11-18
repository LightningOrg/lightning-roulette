//! 🎰 Casino: un bot de Discord hecho con [Lightning] que incluye blackjack,
//! ruleta europea y tragaperras.
//!
//! Ejecútalo con:
//!
//! ```powershell
//! Copy-Item .env.example .env
//! # edita .env y pon tu token
//! cargo run
//! ```
//!
//! La configuración vive en `.env` (ignorado por git):
//!
//! - `DISCORD_TOKEN`: obligatorio.
//! - `DISCORD_PREFIX`: opcional; activa los comandos de prefijo (`!blackjack`).
//! - `DISCORD_GUILD_ID`: opcional; registra los comandos en un servidor concreto
//!   para que aparezcan al instante.
//!
//! Los comandos funcionan con `/`; si defines `DISCORD_PREFIX` (por ejemplo
//! `!`) y activas el intent `MESSAGE_CONTENT` en el portal de desarrolladores
//! de Discord, también funcionan como comandos de prefijo.
//!
//! [Lightning]: https://github.com/ImDaiDev/lightning

mod blackjack;
mod economy;
mod help;
mod roulette;
mod slots;
mod ui;

use lightning::prelude::*;

use blackjack::Games;
use economy::Bank;

#[lightning::main]
async fn main() {
    // rustls 0.23 necesita un proveedor criptográfico explícito. Sin esto, el
    // cliente HTTP (y el gateway) entran en pánico al arrancar.
    let _ = rustls::crypto::ring::default_provider().install_default();

    // Carga `.env` si existe. Las variables ya exportadas en el entorno tienen
    // prioridad (dotenvy no las sobrescribe), así que también vale `$env:`.
    let _ = dotenvy::dotenv();

    let token = std::env::var("DISCORD_TOKEN")
        .map(|token| token.trim().to_owned())
        .ok()
        .filter(|token| !token.is_empty())
        .expect("falta DISCORD_TOKEN: copia .env.example a .env y escribe el token del bot");

    let mut bot = Lightning::new()
        .token(token)
        // Estado compartido por todos los comandos y botones.
        .state(Bank::new())
        .state(Games::new());

    // Los comandos de prefijo (`!blackjack`) necesitan el intent privilegiado
    // MESSAGE_CONTENT, que hay que activar en el portal de Discord. Si el bot
    // no lo tiene, pedirlo cierra la conexión del gateway; por eso el prefijo
    // es opcional y por defecto solo se usan los comandos `/`.
    let prefix = std::env::var("DISCORD_PREFIX")
        .ok()
        .filter(|prefix| !prefix.is_empty());

    if let Some(prefix) = prefix {
        bot = bot.prefix(prefix);
    }

    // Registrar los comandos en un servidor concreto hace que aparezcan al
    // instante (los globales pueden tardar hasta una hora).
    let guild = std::env::var("DISCORD_GUILD_ID")
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok());

    if let Some(id) = guild {
        bot = bot.guild(Id::new(id));
    }

    bot.run().await.expect("el bot se ha caído");
}
