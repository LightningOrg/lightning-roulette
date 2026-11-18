# 🎰 Lightning Roulette

[![CI](https://github.com/ImDaiDev/lightning-roulette/actions/workflows/ci.yml/badge.svg)](https://github.com/ImDaiDev/lightning-roulette/actions/workflows/ci.yml)
[![Rust 1.85+](https://img.shields.io/badge/rust-1.85%2B-orange)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Bot de minijuegos de casino escrito en Rust con
[Lightning](https://github.com/ImDaiDev/lightning): **blackjack**, **ruleta
europea** y **tragaperras**. Funciona como ejemplo y banco de pruebas del
framework: usa comandos, argumentos tipados, estado compartido, botones,
embeds y middleware sin tocar la API de Twilight a mano.

Los comandos se usan con `/`; también admiten prefijo (`!`) si lo activas.

## Minijuegos

| Comando | Qué hace |
| --- | --- |
| `/blackjack apuesta:100` | Reparte una mano contra la banca. Botones para **pedir**, **plantarse**, **doblar** o **retirarse**. El blackjack paga 3:2. |
| `/ruleta apuesta:100 tipo:rojo` | Gira la ruleta europea. Tipos: `rojo`, `negro`, `par`, `impar`, `bajo`, `alto` y `numero`. |
| `/ruleta apuesta:100 tipo:numero numero:17` | Apuesta a un número exacto (paga 36x). |
| `/slots apuesta:100` | Gira tres rodillos. Tres iguales pagan entre 4x y 50x. |
| `/saldo` | Consulta tus fichas. |
| `/casino` | Menú de ayuda. |

Los botones **Jugar otra vez** / **Girar otra vez** repiten la misma apuesta sin
tener que escribir el comando de nuevo.

### Pagos

- **Blackjack:** blackjack natural 3:2; mano ganada 1:1; empate devuelve la
  apuesta. La banca se planta en 17.
- **Ruleta:** color, paridad y mitad pagan 2x (apuesta incluida); un número paga
  36x. El 0 hace perder las apuestas de dinero par.
- **Slots:** pareja 2x, dos sietes 3x, triple de cerezas 4x, limones 5x,
  campanas 8x, estrellas 12x, diamantes 20x y sietes 50x.

Cada jugador empieza con **1.000 fichas**. Las fichas viven en memoria, así que
al reiniciar el bot se reinicia todo el mundo.

## Requisitos

- Rust **1.85+** (edición 2024).
- **CMake** en el `PATH` (Lightning incluye Songbird y compila Opus desde fuente).
- Un bot creado en el [Discord Developer Portal](https://discord.com/developers/applications).

Lightning aún no está publicado en crates.io, así que `Cargo.toml` lo toma
directamente del repositorio de GitHub.

## Configuración

La configuración se lee de un archivo `.env` (ignorado por git) o de variables
de entorno. Empieza copiando la plantilla:

```powershell
Copy-Item .env.example .env
```

| Variable | Obligatoria | Descripción |
| --- | --- | --- |
| `DISCORD_TOKEN` | ✅ | Token del bot (Developer Portal → tu app → **Bot**). |
| `DISCORD_PREFIX` | ❌ | Activa los comandos de prefijo, p. ej. `!`. Requiere el intent **MESSAGE_CONTENT**. |
| `DISCORD_GUILD_ID` | ❌ | ID de un servidor para registrar los comandos ahí al instante. |

Las variables exportadas en el entorno tienen prioridad sobre `.env`.

## Ejecutar

```powershell
cargo run
```

Con `DISCORD_GUILD_ID` los comandos aparecen al momento; sin él se registran de
forma global (Discord puede tardar hasta una hora en mostrarlos).

Invita al bot con los scopes `bot applications.commands` y permisos para ver
canales, enviar mensajes e incrustar embeds.

### Comandos con prefijo (`!`)

Por defecto el bot solo responde a `/`. Para usar `!blackjack 100`:

1. Activa el intent privilegiado **MESSAGE_CONTENT** en el portal de Discord
   (tu app → **Bot** → *Message Content Intent*).
2. Añade `DISCORD_PREFIX=!` a `.env`.

Si pides `MESSAGE_CONTENT` sin tenerlo activado, Discord cierra la conexión del
gateway, así que el prefijo se activa solo cuando lo pides explícitamente.

## Cómo está montado

```text
src/
├── main.rs       # builder: token, prefijo, estado compartido
├── economy.rs    # State<Bank>: saldos y comando /saldo
├── blackjack.rs  # partidas + botones con custom_id
├── roulette.rs   # apuestas y botón de repetir
├── slots.rs      # rodillos y botón de repetir
├── help.rs       # /casino
└── ui.rs         # colores, avisos y respuestas efímeras
```

Qué piezas de Lightning se están probando:

- `#[command]` registra cada comando automáticamente y lo publica en las dos
  superficies (`/` y `!`).
- `CommandArgs` convierte las opciones de Discord y los argumentos posicionales
  en structs tipados.
- `State<T>` inyecta la cartera y las mesas de blackjack sin variables globales.
- `#[component("bj")]` enruta los botones. El estado de la partida viaja dentro
  del `custom_id` (`bj:{"game":1,"action":"hit"}`), y el servidor lo recupera de
  `State<Games>`.
- `Reply`, `Embed`, `ActionRow` y `Button` construyen la respuesta.
- `ui::whisper` usa el cliente HTTP que Lightning deja en `extras` para mandar
  un aviso efímero cuando alguien pulsa el botón de una partida ajena.
- `rustls` 0.23 necesita un proveedor criptográfico: `main` instala el de `ring`
  antes de que Lightning cree el cliente HTTP.

## Tests

La lógica pura (puntuación de manos, colores de la ruleta y tabla de pagos) está
cubierta con tests que no necesitan red ni token:

```powershell
cargo test
```

La integración de los comandos con el framework se puede probar sin Discord con
el módulo `lightning::testing`, pero esos tests viven en el repo del framework.

## CI

`.github/workflows/ci.yml` ejecuta en cada push y pull request:

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- `cargo build --release`

## Seguridad

- `.env` está en `.gitignore`; solo se versiona `.env.example`.
- Nunca subas un token real: si se filtra, réscalo en el Developer Portal
  (*Reset Token*) y actualiza tu `.env`.
- El bot solo necesita los intents no privilegiados que usa por defecto;
  `MESSAGE_CONTENT` es opcional y únicamente para prefijos.

## Licencia

[MIT](LICENSE).
