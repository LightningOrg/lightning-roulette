//! 🎰 Tragaperras de tres rodillos.

use lightning::prelude::*;

use crate::economy::{self, Bank};
use crate::ui;

/// Símbolos de los rodillos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Symbol {
    /// Cereza.
    Cherry,
    /// Limón.
    Lemon,
    /// Campana.
    Bell,
    /// Estrella.
    Star,
    /// Diamante.
    Diamond,
    /// Siete.
    Seven,
}

impl Symbol {
    const ALL: [Symbol; 6] = [
        Symbol::Cherry,
        Symbol::Lemon,
        Symbol::Bell,
        Symbol::Star,
        Symbol::Diamond,
        Symbol::Seven,
    ];

    fn emoji(self) -> &'static str {
        match self {
            Self::Cherry => "🍒",
            Self::Lemon => "🍋",
            Self::Bell => "🔔",
            Self::Star => "⭐",
            Self::Diamond => "💎",
            Self::Seven => "7️⃣",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Cherry => "cerezas",
            Self::Lemon => "limones",
            Self::Bell => "campanas",
            Self::Star => "estrellas",
            Self::Diamond => "diamantes",
            Self::Seven => "sietes",
        }
    }

    fn weight(self) -> u32 {
        match self {
            Self::Cherry => 30,
            Self::Lemon => 25,
            Self::Bell => 18,
            Self::Star => 12,
            Self::Diamond => 9,
            Self::Seven => 6,
        }
    }
}

/// Multiplicador total según la combinación (0 = sin premio).
pub fn multiplier(reels: [Symbol; 3]) -> i64 {
    let [a, b, c] = reels;

    if a == b && b == c {
        return match a {
            Symbol::Seven => 50,
            Symbol::Diamond => 20,
            Symbol::Star => 12,
            Symbol::Bell => 8,
            Symbol::Lemon => 5,
            Symbol::Cherry => 4,
        };
    }

    let sevens = reels
        .iter()
        .filter(|symbol| **symbol == Symbol::Seven)
        .count();
    if sevens == 2 {
        return 3;
    }

    if a == b || a == c || b == c {
        return 2;
    }

    0
}

fn pick() -> Symbol {
    let total: u32 = Symbol::ALL.iter().map(|symbol| symbol.weight()).sum();
    let mut roll = fastrand::u32(0..total);

    for symbol in Symbol::ALL {
        if roll < symbol.weight() {
            return symbol;
        }
        roll -= symbol.weight();
    }

    Symbol::Cherry
}

fn spin() -> [Symbol; 3] {
    [pick(), pick(), pick()]
}

/// Argumentos de `/slots` (también viajan en el botón de repetir).
#[derive(CommandArgs)]
pub struct SlotsArgs {
    /// Fichas que quieres apostar
    apuesta: u64,
}

/// Juega a la tragaperras
#[command(aliases = ["tragaperras", "slot"])]
async fn slots(ctx: Context, bank: State<Bank>, args: Args<SlotsArgs>) -> Reply {
    play(&bank, ctx.user.id.get(), args.apuesta)
}

/// Vuelve a girar con la misma apuesta.
#[component("slots")]
async fn slots_again(ctx: Context, bank: State<Bank>, args: Args<SlotsArgs>) -> Reply {
    play(&bank, ctx.user.id.get(), args.apuesta)
}

fn play(bank: &Bank, user_id: u64, apuesta: u64) -> Reply {
    let bet = match economy::validate_bet(apuesta) {
        Ok(bet) => bet,
        Err(message) => return ui::notice(message),
    };

    if let Err(message) = bank.withdraw(user_id, bet) {
        return ui::notice(message);
    }

    let reels = spin();
    let multiplier = multiplier(reels);
    let won_chips = bet * multiplier;

    if won_chips > 0 {
        bank.deposit(user_id, won_chips);
    }

    let balance = bank.balance(user_id);
    let [a, b, c] = reels;

    let outcome = if multiplier == 0 {
        format!("❌ **Sin premio**\nPierdes **{bet} fichas**")
    } else {
        let combination = if a == b && b == c {
            format!("Triple de {}", a.name())
        } else if reels
            .iter()
            .filter(|symbol| **symbol == Symbol::Seven)
            .count()
            == 2
        {
            "Dos sietes".to_owned()
        } else {
            "Pareja".to_owned()
        };

        format!(
            "✅ **{combination}**\nPaga **x{multiplier}** → **+{} fichas**",
            won_chips - bet
        )
    };

    let embed = Embed::new()
        .title("🎰 Tragaperras")
        .description(format!("**{}　{}　{}**", a.emoji(), b.emoji(), c.emoji()))
        .field("Tu apuesta", format!("**{bet} fichas**"), true)
        .field("Resultado", outcome, true)
        .color(if multiplier == 0 { ui::RED } else { ui::GREEN })
        .footer(format!("Saldo: {balance} fichas"));

    let again = ActionRow::new().button(Button::primary(replay_id(bet), "🔁 Girar otra vez"));

    Reply::new()
        .embed(embed)
        .components(vec![again])
        .no_mentions()
}

fn replay_id(bet: i64) -> String {
    format!(r#"slots:{{"apuesta":{bet}}}"#)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triples_pay_big() {
        assert_eq!(multiplier([Symbol::Seven; 3]), 50);
        assert_eq!(multiplier([Symbol::Diamond; 3]), 20);
        assert_eq!(multiplier([Symbol::Cherry; 3]), 4);
    }

    #[test]
    fn pairs_return_the_bet() {
        assert_eq!(
            multiplier([Symbol::Cherry, Symbol::Cherry, Symbol::Lemon]),
            2
        );
        assert_eq!(multiplier([Symbol::Seven, Symbol::Lemon, Symbol::Seven]), 3);
    }

    #[test]
    fn everything_else_loses() {
        assert_eq!(multiplier([Symbol::Cherry, Symbol::Lemon, Symbol::Bell]), 0);
    }

    #[test]
    fn replay_ids_round_trip() {
        let custom_id = replay_id(250);
        let (name, payload) = custom_id.split_once(':').expect("nombre:json");
        assert_eq!(name, "slots");

        let options: lightning::serde_json::Map<String, lightning::serde_json::Value> =
            lightning::serde_json::from_str(payload).expect("el payload es JSON");
        let args = <SlotsArgs as CommandArgs>::from_options(options).expect("args válidos");

        assert_eq!(args.apuesta, 250);
    }
}
