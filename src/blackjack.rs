//! 🃏 Blackjack contra la banca.
//!
//! Cada mano es un `Game` guardado en [`Games`]. Los botones llevan el id de la
//! partida dentro del `custom_id`, así que la interacción no necesita recordar
//! nada entre pulsaciones.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};

use lightning::prelude::*;

use crate::economy::{self, Bank};
use crate::ui;

/// Cuántas barajas tiene el zapato.
const DECKS: usize = 4;

// ---------------------------------------------------------------------------
// Cartas y puntuación
// ---------------------------------------------------------------------------

/// Palo de una carta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Suit {
    /// Tréboles.
    Clubs,
    /// Diamantes.
    Diamonds,
    /// Corazones.
    Hearts,
    /// Picas.
    Spades,
}

impl Suit {
    const ALL: [Suit; 4] = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades];

    fn symbol(self) -> &'static str {
        match self {
            Self::Clubs => "♣",
            Self::Diamonds => "♦",
            Self::Hearts => "♥",
            Self::Spades => "♠",
        }
    }
}

/// Una carta: rango de 1 (as) a 13 (rey).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Card {
    /// 1 = as, 11 = J, 12 = Q, 13 = K.
    pub rank: u8,
    /// Palo.
    pub suit: Suit,
}

impl Card {
    fn label(self) -> String {
        let rank = match self.rank {
            1 => "A".to_owned(),
            11 => "J".to_owned(),
            12 => "Q".to_owned(),
            13 => "K".to_owned(),
            other => other.to_string(),
        };

        format!("{rank}{}", self.suit.symbol())
    }

    fn value(self) -> u8 {
        match self.rank {
            1 => 11,
            11..=13 => 10,
            other => other,
        }
    }
}

/// Valor de una mano, contando los ases como 11 o 1 según convenga.
pub fn hand_value(cards: &[Card]) -> u8 {
    let mut total: u8 = 0;
    let mut aces = 0u8;

    for card in cards {
        total += card.value();
        if card.rank == 1 {
            aces += 1;
        }
    }

    while total > 21 && aces > 0 {
        total -= 10;
        aces -= 1;
    }

    total
}

fn is_blackjack(cards: &[Card]) -> bool {
    cards.len() == 2 && hand_value(cards) == 21
}

fn cards_text(cards: &[Card]) -> String {
    cards
        .iter()
        .map(|card| card.label())
        .collect::<Vec<_>>()
        .join(" ")
}

fn shoe() -> Vec<Card> {
    let mut cards = Vec::with_capacity(52 * DECKS);

    for _ in 0..DECKS {
        for suit in Suit::ALL {
            for rank in 1..=13 {
                cards.push(Card { rank, suit });
            }
        }
    }

    fastrand::shuffle(&mut cards);
    cards
}

// ---------------------------------------------------------------------------
// Resultado
// ---------------------------------------------------------------------------

/// Cómo acaba una mano.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Blackjack natural (21 con dos cartas).
    Blackjack,
    /// Gana el jugador.
    Win,
    /// Empate: se devuelve la apuesta.
    Push,
    /// Gana la banca.
    Lose,
}

/// Decide el resultado de una mano ya terminada.
pub fn outcome_of(player: &[Card], dealer: &[Card]) -> Outcome {
    let player_value = hand_value(player);
    let dealer_value = hand_value(dealer);

    if player_value > 21 {
        return Outcome::Lose;
    }
    if dealer_value > 21 {
        return Outcome::Win;
    }

    let player_blackjack = is_blackjack(player);
    let dealer_blackjack = is_blackjack(dealer);

    if player_blackjack && dealer_blackjack {
        return Outcome::Push;
    }
    if player_blackjack {
        return Outcome::Blackjack;
    }
    if dealer_blackjack {
        return Outcome::Lose;
    }

    match player_value.cmp(&dealer_value) {
        std::cmp::Ordering::Greater => Outcome::Win,
        std::cmp::Ordering::Less => Outcome::Lose,
        std::cmp::Ordering::Equal => Outcome::Push,
    }
}

/// Fichas que vuelven a la cartera (apuesta incluida).
pub fn payout_for(outcome: Outcome, bet: i64) -> i64 {
    match outcome {
        // 3:2 de beneficio.
        Outcome::Blackjack => bet + bet * 3 / 2,
        Outcome::Win => bet * 2,
        Outcome::Push => bet,
        Outcome::Lose => 0,
    }
}

// ---------------------------------------------------------------------------
// Partida
// ---------------------------------------------------------------------------

/// Estado de una mano en curso.
#[derive(Debug, Clone)]
pub struct Game {
    /// Identificador que viaja en los botones.
    pub id: u64,
    /// Dueño de la mano.
    pub user_id: u64,
    /// Apuesta actual (se duplica al doblar).
    pub bet: i64,
    deck: Vec<Card>,
    /// Cartas del jugador.
    pub player: Vec<Card>,
    /// Cartas de la banca.
    pub dealer: Vec<Card>,
    /// `true` si el jugador dobló.
    pub doubled: bool,
    /// `true` cuando ya no se admiten más acciones.
    pub finished: bool,
    /// Resultado, si la mano terminó.
    pub outcome: Option<Outcome>,
    /// Fichas que hay que devolver a la cartera.
    pub payout: i64,
}

impl Game {
    fn new(id: u64, user_id: u64, bet: i64) -> Self {
        let mut deck = shoe();
        let player = vec![
            deck.pop().expect("el zapato nunca se vacía"),
            deck.pop().expect("el zapato nunca se vacía"),
        ];
        let dealer = vec![
            deck.pop().expect("el zapato nunca se vacía"),
            deck.pop().expect("el zapato nunca se vacía"),
        ];

        let mut game = Self {
            id,
            user_id,
            bet,
            deck,
            player,
            dealer,
            doubled: false,
            finished: false,
            outcome: None,
            payout: 0,
        };

        // Un blackjack natural se resuelve al instante.
        if is_blackjack(&game.player) || is_blackjack(&game.dealer) {
            game.finish();
        }

        game
    }

    fn draw(&mut self) -> Card {
        self.deck.pop().expect("el zapato nunca se vacía")
    }

    /// Pide una carta. Si se pasa de 21, la mano termina.
    pub fn hit(&mut self) {
        if self.finished {
            return;
        }

        let card = self.draw();
        self.player.push(card);

        if hand_value(&self.player) > 21 {
            self.finish();
        }
    }

    /// La banca juega (se planta en 17) y se calcula el resultado.
    pub fn finish(&mut self) {
        if self.finished {
            return;
        }

        let player_value = hand_value(&self.player);
        if player_value <= 21 && !is_blackjack(&self.player) {
            while hand_value(&self.dealer) < 17 {
                let card = self.draw();
                self.dealer.push(card);
            }
        }

        let outcome = outcome_of(&self.player, &self.dealer);
        self.outcome = Some(outcome);
        self.payout = payout_for(outcome, self.bet);
        self.finished = true;
    }

    /// Abandona la mano: se pierde la apuesta pero se libera la mesa.
    pub fn fold(&mut self) {
        if self.finished {
            return;
        }

        self.outcome = Some(Outcome::Lose);
        self.payout = 0;
        self.finished = true;
    }
}

/// Partidas activas.
#[derive(Default)]
pub struct Games {
    next_id: AtomicU64,
    games: Mutex<HashMap<u64, Game>>,
}

impl Games {
    /// Almacén vacío.
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<u64, Game>> {
        self.games
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    fn insert(&self, game: Game) {
        self.lock().insert(game.id, game);
    }

    fn remove(&self, id: u64) {
        self.lock().remove(&id);
    }

    fn has_active(&self, user_id: u64) -> bool {
        self.lock()
            .values()
            .any(|game| game.user_id == user_id && !game.finished)
    }
}

// ---------------------------------------------------------------------------
// Presentación
// ---------------------------------------------------------------------------

fn can_double(game: &Game, bank: &Bank) -> bool {
    !game.finished && game.player.len() == 2 && bank.balance(game.user_id) >= game.bet
}

fn action_id(game_id: u64, action: &str) -> String {
    format!(r#"bj:{{"game":{game_id},"action":"{action}"}}"#)
}

fn render(game: &Game, balance: i64, can_double: bool) -> Reply {
    let player_value = hand_value(&game.player);
    let doubled = if game.doubled { " (doblada)" } else { "" };

    let mut embed = Embed::new()
        .title("♠️ Blackjack")
        .description(format!(
            "Mano de <@{}>\nApuesta: **{} fichas**{}",
            game.user_id, game.bet, doubled
        ))
        .field(
            "Tu mano",
            format!("{}\n**{player_value} puntos**", cards_text(&game.player)),
            false,
        );

    if let Some(outcome) = game.outcome.filter(|_| game.finished) {
        let dealer_value = hand_value(&game.dealer);
        embed = embed
            .field(
                "La banca",
                format!("{}\n**{dealer_value} puntos**", cards_text(&game.dealer)),
                false,
            )
            .field("Resultado", result_text(game, outcome), false)
            .color(match outcome {
                Outcome::Blackjack => ui::GOLD,
                Outcome::Win => ui::GREEN,
                Outcome::Push => ui::SLATE,
                Outcome::Lose => ui::RED,
            });
    } else {
        let visible = &game.dealer[..1];
        embed = embed
            .field(
                "La banca",
                format!(
                    "{} 🂠\n**{} puntos** + ?",
                    cards_text(visible),
                    hand_value(visible)
                ),
                false,
            )
            .color(ui::BLURPLE);
    }

    embed = embed.footer(format!("Saldo: {balance} fichas"));

    let mut reply = Reply::new().embed(embed).no_mentions();

    if !game.finished {
        let buttons = ActionRow::new()
            .button(Button::success(action_id(game.id, "hit"), "🃏 Pedir"))
            .button(Button::primary(action_id(game.id, "stand"), "✋ Plantarse"))
            .button(
                Button::secondary(action_id(game.id, "double"), "💥 Doblar").disabled(!can_double),
            )
            .button(Button::danger(action_id(game.id, "fold"), "🏳️ Retirarse"));

        reply = reply.components(vec![buttons]);
    }

    reply
}

fn result_text(game: &Game, outcome: Outcome) -> String {
    let profit = game.payout - game.bet;

    match outcome {
        Outcome::Blackjack => format!("✨ **¡Blackjack!**\nGanas **+{profit} fichas** (3:2)"),
        Outcome::Win => format!("✅ **Ganas**\n**+{profit} fichas**"),
        Outcome::Push => "🤝 **Empate**\nRecuperas tu apuesta".to_owned(),
        Outcome::Lose => format!("❌ **Pierdes**\n**-{} fichas**", game.bet),
    }
}

// ---------------------------------------------------------------------------
// Comandos
// ---------------------------------------------------------------------------

/// Argumentos de `/blackjack`.
#[derive(CommandArgs)]
pub struct BlackjackArgs {
    /// Fichas que quieres apostar
    apuesta: u64,
}

/// Juega una mano de blackjack contra la banca
#[command(aliases = ["bj", "21"])]
async fn blackjack(
    ctx: Context,
    bank: State<Bank>,
    games: State<Games>,
    args: Args<BlackjackArgs>,
) -> Reply {
    let user_id = ctx.user.id.get();

    let bet = match economy::validate_bet(args.apuesta) {
        Ok(bet) => bet,
        Err(message) => return ui::notice(message),
    };

    if games.has_active(user_id) {
        return ui::notice("🃏 Ya tienes una mano en curso. Termínala antes de empezar otra.");
    }

    if let Err(message) = bank.withdraw(user_id, bet) {
        return ui::notice(message);
    }

    let game = Game::new(games.next_id(), user_id, bet);

    if game.finished {
        // Blackjack natural nada más repartir.
        bank.deposit(user_id, game.payout);
    } else {
        games.insert(game.clone());
    }

    let balance = bank.balance(user_id);
    render(&game, balance, can_double(&game, &bank))
}

/// Datos que viajan en el `custom_id` de cada botón.
#[derive(CommandArgs)]
struct ButtonArgs {
    /// Id de la partida
    game: u64,
    /// Acción: hit, stand o double
    action: String,
}

/// Por qué un botón no se pudo atender.
enum ButtonError {
    Gone,
    NotOwner,
    Finished,
    TooLateToDouble,
    NotEnoughChips,
    Bank(String),
    Unknown,
}

/// Maneja los botones de todas las mesas de blackjack.
///
/// El candado de las partidas no puede cruzar un `.await` (el guard de
/// `std::sync::Mutex` no es `Send`), así que toda la parte bloqueante se hace
/// dentro de un bloque y los avisos efímeros se envían después.
#[component("bj")]
async fn blackjack_button(
    ctx: Context,
    bank: State<Bank>,
    games: State<Games>,
    args: Args<ButtonArgs>,
) -> Reply {
    let user_id = ctx.user.id.get();
    let id = args.game;
    let action = args.action.to_ascii_lowercase();

    let result = {
        let mut table = games.lock();

        match table.get_mut(&id) {
            None => Err(ButtonError::Gone),
            Some(game) if game.user_id != user_id => Err(ButtonError::NotOwner),
            Some(game) if game.finished => Err(ButtonError::Finished),
            Some(game) => {
                let action_result = match action.as_str() {
                    "hit" => {
                        game.hit();
                        Ok(())
                    }
                    "stand" => {
                        game.finish();
                        Ok(())
                    }
                    "fold" => {
                        game.fold();
                        Ok(())
                    }
                    "double" => {
                        if game.player.len() != 2 {
                            Err(ButtonError::TooLateToDouble)
                        } else if bank.balance(user_id) < game.bet {
                            Err(ButtonError::NotEnoughChips)
                        } else if let Err(message) = bank.withdraw(user_id, game.bet) {
                            Err(ButtonError::Bank(message))
                        } else {
                            game.bet *= 2;
                            game.doubled = true;

                            let card = game.draw();
                            game.player.push(card);
                            game.finish();
                            Ok(())
                        }
                    }
                    _ => Err(ButtonError::Unknown),
                };

                match action_result {
                    Err(error) => Err(error),
                    Ok(()) => {
                        if game.finished {
                            bank.deposit(game.user_id, game.payout);
                        }

                        let balance = bank.balance(user_id);
                        let reply = render(game, balance, can_double(game, &bank));
                        Ok((reply, game.finished))
                    }
                }
            }
        }
    };

    match result {
        Ok((reply, finished)) => {
            if finished {
                games.remove(id);
            }
            reply
        }
        Err(ButtonError::Gone) => {
            ui::whisper(&ctx, "🕐 Esa mano ya ha terminado.").await;
            Reply::new()
        }
        Err(ButtonError::NotOwner) => {
            ui::whisper(
                &ctx,
                "❌ Esta mano no es tuya. Abre la tuya con `/blackjack`.",
            )
            .await;
            Reply::new()
        }
        Err(ButtonError::Finished) => ui::notice("🕐 Esta mano ya ha terminado."),
        Err(ButtonError::TooLateToDouble) => {
            ui::whisper(&ctx, "💥 Solo puedes doblar con las dos primeras cartas.").await;
            Reply::new()
        }
        Err(ButtonError::NotEnoughChips) => {
            ui::whisper(&ctx, "❌ No tienes fichas suficientes para doblar.").await;
            Reply::new()
        }
        Err(ButtonError::Bank(message)) => {
            ui::whisper(&ctx, message).await;
            Reply::new()
        }
        Err(ButtonError::Unknown) => {
            ui::whisper(&ctx, "Esa acción no existe.").await;
            Reply::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(rank: u8) -> Card {
        Card {
            rank,
            suit: Suit::Spades,
        }
    }

    #[test]
    fn aces_count_as_one_or_eleven() {
        assert_eq!(hand_value(&[card(1), card(1), card(9)]), 21);
        assert_eq!(hand_value(&[card(1), card(10), card(10)]), 21);
        assert_eq!(hand_value(&[card(1), card(1), card(1), card(1)]), 14);
    }

    #[test]
    fn natural_blackjack_wins_three_to_two() {
        let player = [card(1), card(13)];
        let dealer = [card(7), card(7), card(7)];

        assert_eq!(outcome_of(&player, &dealer), Outcome::Blackjack);
        assert_eq!(payout_for(Outcome::Blackjack, 100), 250);
    }

    #[test]
    fn two_naturals_push() {
        let player = [card(1), card(12)];
        let dealer = [card(1), card(11)];

        assert_eq!(outcome_of(&player, &dealer), Outcome::Push);
    }

    #[test]
    fn bust_loses_even_if_the_dealer_would_bust() {
        let player = [card(10), card(6), card(9)];
        let dealer = [card(10), card(6), card(9)];

        assert_eq!(outcome_of(&player, &dealer), Outcome::Lose);
    }

    #[test]
    fn button_ids_round_trip() {
        let custom_id = action_id(7, "double");
        let (name, payload) = custom_id.split_once(':').expect("nombre:json");
        assert_eq!(name, "bj");

        let options: lightning::serde_json::Map<String, lightning::serde_json::Value> =
            lightning::serde_json::from_str(payload).expect("el payload es JSON");
        let args = <ButtonArgs as CommandArgs>::from_options(options).expect("args válidos");

        assert_eq!(args.game, 7);
        assert_eq!(args.action, "double");
    }
}
