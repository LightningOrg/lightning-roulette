//! 🎡 Ruleta europea (un solo cero).

// `is_multiple_of` pide Rust 1.87 y el workspace declara 1.85.
#![allow(clippy::manual_is_multiple_of)]

use lightning::prelude::*;

use crate::economy::{self, Bank};
use crate::ui;

const REDS: [u8; 18] = [
    1, 3, 5, 7, 9, 12, 14, 16, 18, 19, 21, 23, 25, 27, 30, 32, 34, 36,
];

/// `true` si el número es rojo en la ruleta europea.
pub fn is_red(number: u8) -> bool {
    REDS.contains(&number)
}

/// A qué se apuesta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BetKind {
    /// Números rojos.
    Red,
    /// Números negros.
    Black,
    /// Pares (el 0 no cuenta).
    Even,
    /// Impares.
    Odd,
    /// Del 1 al 18.
    Low,
    /// Del 19 al 36.
    High,
    /// Un número exacto.
    Number(u8),
}

impl BetKind {
    /// `true` si esta apuesta gana con ese resultado.
    pub fn wins(self, result: u8) -> bool {
        match self {
            Self::Red => result != 0 && is_red(result),
            Self::Black => result != 0 && !is_red(result),
            Self::Even => result != 0 && result % 2 == 0,
            Self::Odd => result != 0 && result % 2 == 1,
            Self::Low => (1..=18).contains(&result),
            Self::High => (19..=36).contains(&result),
            Self::Number(number) => result == number,
        }
    }

    /// Multiplicador total (apuesta incluida).
    pub fn multiplier(self) -> i64 {
        match self {
            Self::Number(_) => 36,
            _ => 2,
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::Red => "rojo",
            Self::Black => "negro",
            Self::Even => "par",
            Self::Odd => "impar",
            Self::Low => "bajo",
            Self::High => "alto",
            Self::Number(_) => "numero",
        }
    }

    fn label(self) -> String {
        match self {
            Self::Number(number) => format!("número {number}"),
            other => other.key().to_owned(),
        }
    }
}

/// Fichas que se pagan por una apuesta acertada (0 si se pierde).
pub fn payout(kind: BetKind, bet: i64, result: u8) -> i64 {
    if kind.wins(result) {
        bet * kind.multiplier()
    } else {
        0
    }
}

fn parse_bet(tipo: &str, numero: Option<u8>) -> std::result::Result<BetKind, String> {
    let tipo = tipo.trim().to_ascii_lowercase();

    let kind = match tipo.as_str() {
        "rojo" | "red" => BetKind::Red,
        "negro" | "black" => BetKind::Black,
        "par" | "even" => BetKind::Even,
        "impar" | "odd" => BetKind::Odd,
        "bajo" | "low" => BetKind::Low,
        "alto" | "high" => BetKind::High,
        "numero" | "número" | "number" | "n" => {
            let number = numero.ok_or_else(|| {
                "Para apostar a un número escribe también `numero:<0-36>`.".to_owned()
            })?;

            if number > 36 {
                return Err("El número tiene que estar entre 0 y 36.".to_owned());
            }

            BetKind::Number(number)
        }
        other => {
            return Err(format!(
                "❌ Tipo de apuesta desconocido: `{other}`.\n\
                 Usa `rojo`, `negro`, `par`, `impar`, `bajo`, `alto` o `numero`."
            ));
        }
    };

    if numero.is_some() && !matches!(kind, BetKind::Number(_)) {
        return Err(
            "El número solo se usa con `tipo:numero` (por ejemplo `tipo:numero numero:17`)."
                .to_owned(),
        );
    }

    Ok(kind)
}

/// Argumentos de `/ruleta` (también viajan en el botón de repetir).
#[derive(CommandArgs)]
pub struct RouletteArgs {
    /// Fichas que quieres apostar
    apuesta: u64,
    /// rojo, negro, par, impar, bajo, alto o numero
    tipo: String,
    /// Número del 0 al 36 (solo con tipo = numero)
    numero: Option<u8>,
}

/// Juega a la ruleta europea
#[command(aliases = ["roulette", "rl"])]
async fn ruleta(ctx: Context, bank: State<Bank>, args: Args<RouletteArgs>) -> Reply {
    play(&bank, ctx.user.id.get(), &args.0)
}

/// Repite la misma apuesta al pulsar el botón.
#[component("ruleta")]
async fn ruleta_again(ctx: Context, bank: State<Bank>, args: Args<RouletteArgs>) -> Reply {
    play(&bank, ctx.user.id.get(), &args.0)
}

fn play(bank: &Bank, user_id: u64, args: &RouletteArgs) -> Reply {
    let kind = match parse_bet(&args.tipo, args.numero) {
        Ok(kind) => kind,
        Err(message) => return ui::notice(message),
    };

    let bet = match economy::validate_bet(args.apuesta) {
        Ok(bet) => bet,
        Err(message) => return ui::notice(message),
    };

    if let Err(message) = bank.withdraw(user_id, bet) {
        return ui::notice(message);
    }

    spin(bank, user_id, bet, kind)
}

fn spin(bank: &Bank, user_id: u64, bet: i64, kind: BetKind) -> Reply {
    let number = fastrand::u8(0..=36);
    let won = kind.wins(number);
    let won_chips = payout(kind, bet, number);

    if won_chips > 0 {
        bank.deposit(user_id, won_chips);
    }

    let balance = bank.balance(user_id);
    let (dot, color) = match number {
        0 => ("🟢", ui::GREEN),
        n if is_red(n) => ("🔴", ui::RED),
        _ => ("⚫", ui::SLATE),
    };

    let color_name = match number {
        0 => "verde",
        n if is_red(n) => "rojo",
        _ => "negro",
    };
    let parity = if number == 0 {
        "ni par ni impar"
    } else if number % 2 == 0 {
        "par"
    } else {
        "impar"
    };
    let range = match number {
        0 => "la casa",
        1..=18 => "bajo (1-18)",
        _ => "alto (19-36)",
    };

    let outcome = if won {
        format!("✅ **Ganas +{} fichas**", won_chips - bet)
    } else {
        format!("❌ **Pierdes -{bet} fichas**")
    };

    let embed = Embed::new()
        .title("🎡 Ruleta europea")
        .description(format!(
            "{dot} **{number}**\n{color_name} · {parity} · {range}"
        ))
        .field(
            "Tu apuesta",
            format!("**{bet} fichas** a **{}**", kind.label()),
            true,
        )
        .field("Resultado", outcome, true)
        .color(color)
        .footer(format!("Saldo: {balance} fichas"));

    let again = ActionRow::new().button(Button::primary(replay_id(bet, kind), "🔁 Jugar otra vez"));

    Reply::new()
        .embed(embed)
        .components(vec![again])
        .no_mentions()
}

fn replay_id(bet: i64, kind: BetKind) -> String {
    match kind {
        BetKind::Number(number) => {
            format!(r#"ruleta:{{"apuesta":{bet},"tipo":"numero","numero":{number}}}"#)
        }
        other => format!(
            r#"ruleta:{{"apuesta":{bet},"tipo":"{}","numero":null}}"#,
            other.key()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn red_numbers_match_the_european_wheel() {
        assert!(is_red(1));
        assert!(is_red(36));
        assert!(!is_red(2));
        assert!(!is_red(0));
    }

    #[test]
    fn zero_kills_even_money_bets() {
        for kind in [
            BetKind::Red,
            BetKind::Black,
            BetKind::Even,
            BetKind::Odd,
            BetKind::Low,
            BetKind::High,
        ] {
            assert!(!kind.wins(0));
            assert_eq!(payout(kind, 100, 0), 0);
        }

        assert!(BetKind::Number(0).wins(0));
    }

    #[test]
    fn numbers_pay_thirty_six_times() {
        assert_eq!(payout(BetKind::Number(17), 100, 17), 3_600);
        assert_eq!(payout(BetKind::Red, 100, 3), 200);
        assert_eq!(payout(BetKind::Black, 100, 3), 0);
        assert_eq!(payout(BetKind::Low, 100, 18), 200);
        assert_eq!(payout(BetKind::High, 100, 18), 0);
    }

    #[test]
    fn replay_ids_round_trip() {
        let custom_id = replay_id(100, BetKind::Number(17));
        let (name, payload) = custom_id.split_once(':').expect("nombre:json");
        assert_eq!(name, "ruleta");

        let options: lightning::serde_json::Map<String, lightning::serde_json::Value> =
            lightning::serde_json::from_str(payload).expect("el payload es JSON");
        let args = <RouletteArgs as CommandArgs>::from_options(options).expect("args válidos");

        assert_eq!(args.apuesta, 100);
        assert_eq!(args.tipo, "numero");
        assert_eq!(args.numero, Some(17));
    }
}
