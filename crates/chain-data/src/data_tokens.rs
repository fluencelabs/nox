/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use crate::ChainDataError;
use crate::ChainDataError::{InvalidParsedToken, MissingParsedToken};
use ethabi::Token;

// Take next token and parse it with `f`
pub fn next_opt<T>(
    data_tokens: &mut impl Iterator<Item = Token>,
    name: &'static str,
    f: impl Fn(Token) -> Option<T>,
) -> Result<T, ChainDataError> {
    let next = data_tokens.next().ok_or(MissingParsedToken(name))?;
    let parsed = f(next).ok_or(InvalidParsedToken(name))?;

    Ok(parsed)
}

// Take next token and parse it with `f`
pub fn next<T>(
    data_tokens: &mut impl Iterator<Item = Token>,
    name: &'static str,
    f: impl Fn(Token) -> T,
) -> Result<T, ChainDataError> {
    next_opt(data_tokens, name, |t| Some(f(t)))
}
