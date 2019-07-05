// *************************************************************************
// * Copyright (C) 2019 Dmitry Narkevich (me@dmitry.lol)                   *
// *                                                                       *
// * This program is free software: you can redistribute it and/or modify  *
// * it under the terms of the GNU General Public License as published by  *
// * the Free Software Foundation, either version 3 of the License, or     *
// * (at your option) any later version.                                   *
// *                                                                       *
// * This program is distributed in the hope that it will be useful,       *
// * but WITHOUT ANY WARRANTY; without even the implied warranty of        *
// * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the         *
// * GNU General Public License for more details.                          *
// *                                                                       *
// * You should have received a copy of the GNU General Public License     *
// * along with this program.  If not, see <http://www.gnu.org/licenses/>. *
// *************************************************************************

use crate::*;

pub fn truncate_from_end(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.into()
    } else {
        let delta = s.len() - n; // how many characters don't fit
        let truncd = &s[delta..];
        let non_dots = &truncd[min(3, truncd.len())..]; // part of string that doesn't get turned into dots
        let with_dots = format!("...{}", non_dots);
        with_dots[with_dots.len().checked_sub(n).unwrap_or(0)..].into()
    }
}

pub fn is_default<T: Default + PartialEq>(t: &T) -> bool {
    t == &T::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_trunc() {
        assert_eq!(truncate_from_end("abcd", 5), "abcd");
        assert_eq!(truncate_from_end("abcd", 4), "abcd");
        assert_eq!(truncate_from_end("", 4), "");
    }

    #[test]
    fn trunc() {
        assert_eq!(truncate_from_end("abcd", 0), "");
        assert_eq!(truncate_from_end("abcd", 1), ".");
        assert_eq!(truncate_from_end("ab", 1), ".");
        assert_eq!(truncate_from_end("abcd", 2), "..");
        assert_eq!(truncate_from_end("abcd", 3), "...");
        assert_eq!(truncate_from_end("abcde", 4), "...e");
        assert_eq!(truncate_from_end("the quick brown", 8), "...brown");

    }
}

