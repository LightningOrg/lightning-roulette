//! Cartera de fichas compartida por todos los minijuegos.
//!
//! Las fichas viven en memoria: al reiniciar el bot todo el mundo vuelve a
//! empezar con [`STARTING_CHIPS`]. Para un casino de verdad esto sería una base
//! de datos, pero para un ejemplo así se ve el uso de `State` sin ruido.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

use lightning::prelude::*;

/// Fichas con las que empieza cada jugador.
pub const STARTING_CHIPS: i64 = 1_000;

/// Apuesta máxima por jugada.
pub const MAX_BET: i64 = 1_000_000;

/// Saldos por id de usuario.
#[derive(Default)]
pub struct Bank {
    balances: Mutex<HashMap<u64, i64>>,
}

impl Bank {
    /// Cartera vacía: los saldos se crean al vuelo.
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<u64, i64>> {
        self.balances
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Saldo actual (o el inicial si es la primera vez que aparece el usuario).
    pub fn balance(&self, user_id: u64) -> i64 {
        *self.lock().entry(user_id).or_insert(STARTING_CHIPS)
    }

    /// Retira la apuesta. Devuelve un mensaje listo para enseñar al usuario si
    /// no se puede apostar.
    pub fn withdraw(&self, user_id: u64, amount: i64) -> std::result::Result<(), String> {
        if amount <= 0 {
            return Err("La apuesta tiene que ser de al menos **1 ficha**.".to_owned());
        }
        if amount > MAX_BET {
            return Err(format!("La apuesta máxima es de **{MAX_BET} fichas**."));
        }

        let mut balances = self.lock();
        let balance = balances.entry(user_id).or_insert(STARTING_CHIPS);

        if *balance < amount {
            return Err(format!(
                "❌ No tienes suficientes fichas: te quedan **{balance}** y querías apostar **{amount}**."
            ));
        }

        *balance -= amount;
        Ok(())
    }

    /// Ingresa un premio (incluida la devolución de la apuesta).
    pub fn deposit(&self, user_id: u64, amount: i64) {
        if amount <= 0 {
            return;
        }

        *self.lock().entry(user_id).or_insert(STARTING_CHIPS) += amount;
    }
}

/// Convierte la apuesta que llega de Discord en un importe válido.
pub fn validate_bet(amount: u64) -> std::result::Result<i64, String> {
    if amount == 0 {
        return Err("La apuesta tiene que ser de al menos **1 ficha**.".to_owned());
    }
    if amount > MAX_BET as u64 {
        return Err(format!("La apuesta máxima es de **{MAX_BET} fichas**."));
    }

    Ok(amount as i64)
}

/// Muestra cuántas fichas tienes.
#[command(aliases = ["balance", "cartera", "fichas"])]
async fn saldo(ctx: Context, bank: State<Bank>) -> Reply {
    let balance = bank.balance(ctx.user.id.get());

    Reply::new()
        .embed(
            Embed::new()
                .title("💰 Tu cartera")
                .description(format!("Tienes **{balance} fichas**."))
                .color(crate::ui::GOLD)
                .footer("Puedes ganar más en /blackjack, /ruleta y /slots."),
        )
        .no_mentions()
}
