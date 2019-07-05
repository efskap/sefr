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

pub trait SuggestionAdapter {
    fn get(url:String, term: String) -> Result<Suggestions, Box<std::error::Error>>;

}
pub struct OpenSearchAdapter;
impl SuggestionAdapter for OpenSearchAdapter {
    fn get(url: String, _term: String) -> Result<Suggestions, Box<std::error::Error>> {
        let text = minreq::get(url).send()?.body;
        let data = json::parse(&text)?;
        let term = data[0]
            .as_str()
            .ok_or("first array value not a string")?
            .to_string();
        let sugg_terms: Vec<String> = data[1].members().map(|opt|opt.as_str().expect("one of the values in the second value (which should be an array) is not a string").to_string()).collect(); // todo: error handling
        Ok(Suggestions { term, sugg_terms })
    }
}
pub struct JsonPathAdapter(pub String);
impl JsonPathAdapter {
    pub fn get(self, url: String, term: String) -> Result<Suggestions, Box<std::error::Error>> {
        let text = minreq::get(url).send()?.body;
        let data = json::parse(&text)?;
        let path = self.0.split('.');
        let mut obj = data;

        for segment in path {
            obj = obj[segment].take();
        }

        if !obj.is_array() {
            return Err(Box::new(ConfigError::new(&format!("JsonPath '{}' doesn't lead to an array!", &self.0))));
        }

        let sugg_terms: Vec<String> = obj.members().map(|opt|opt.as_str().expect("non-string value under jsonpath target").to_string()).collect(); // todo: error handling
        Ok(Suggestions { term, sugg_terms })
    }
}
