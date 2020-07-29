#![warn(missing_docs)]
//! A couple useful thiings for my Discord bots

use std::mem::replace;
use std::vec::IntoIter as VecIntoIter;

/// The Discord character limit for a message
pub const MSG_LIMIT: usize = 2000;

#[derive(Debug, Default, Clone)]
/// A collection of strings which are all within the characters limit
pub struct MsgBunch {
    messages: Vec<String>,
}

impl MsgBunch {
    fn new() -> Self {
        MsgBunch {
            messages: vec![String::with_capacity(MSG_LIMIT)]
        }
    }

    /// Shorthand for `MsgBunchBuilder::new()`
    #[inline(always)]
    pub fn builder() -> MsgBunchBuilder {
        MsgBunchBuilder::new()
    }

    /// Consumes the `MsgBunch` and returns the inner vector of strings
    pub fn into_inner(self) -> Vec<String> {
        self.messages
    }
}

impl IntoIterator for MsgBunch {
    type IntoIter = VecIntoIter<String>;
    type Item = String;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.messages.into_iter()
    }
}

#[derive(Debug)]
/// Structure for making an `MsgBunch` that allows control over where the messsage can be split
pub struct MsgBunchBuilder {
    /// the inner `MsgBunch` being worked on
    /// will not contain the current split section
    /// use `build` to make sure you get the full thing
    pub inner: MsgBunch,
    chars_num: usize, 
    no_split_section: Option<(String, usize)>,
}

impl Default for MsgBunchBuilder {
    #[inline(always)]
    fn default() -> Self {
        MsgBunchBuilder::new()
    }
}

impl MsgBunchBuilder {
    #[inline]
    /// Begin making an `MsgBunch`
    pub fn new() -> Self {
        MsgBunchBuilder {
            inner: MsgBunch::new(),
            chars_num: 0,
            no_split_section: None,
        }
    }

    /// Adds a string to the `MsgBunch` splitting if necessary
    /// This changes the way it splits depending on whether it is currently in a section.
    ///
    /// Sections are started with `MsgBunchBuilder::begin_section` and ended with `MsgBunchBuilder::end_section`
    ///
    /// # Example
    ///
    /// For example you tell the builder to try not to split a welcome message.
    /// It will then try its best not to split in the middle of that section, putting it into its own message if need be.
    ///
    /// ```
    /// # fn get_name() -> String { "Jens".to_owned() }
    /// # fn motd() -> &'static str {""}
    /// use discord_utils::MsgBunchBuilder;
    /// 
    /// let mut mmb = MsgBunchBuilder::new();
    /// mmb
    ///     .begin_section()
    ///     .add_string("Hello, ")
    ///     // Some sort of function that gets the name of the user
    ///     .add_string(get_name())
    ///     .add_string("!\n")
    ///     .end_section()
    ///     // Perhaps the message ends with an motd, that contains a lot of lines.
    ///     .add_lines(motd());
    /// 
    /// let msg_bunch = mmb.build();
    /// ```
    pub fn add_string<S: AsRef<str>>(&mut self, s: S) -> &mut Self {
        let string_to_add = s.as_ref();
        let string_to_add_size = string_to_add.chars().count();

        if let Some((no_split_section, size)) = &mut self.no_split_section {
            *size += string_to_add_size;
            no_split_section.push_str(string_to_add);
        } else if self.chars_num + string_to_add_size > MSG_LIMIT {
            let cur_msg = self.inner.messages.last_mut().unwrap();
            let cur_msg_size = cur_msg.chars().count();

            let (s, index) = (cur_msg_size+1..).zip(string_to_add.char_indices()).map(|(s, (i, _))| (s, i)).nth(MSG_LIMIT-cur_msg_size).unwrap();
            debug_assert_eq!(s, MSG_LIMIT);

            cur_msg.push_str(&string_to_add[..index]);

            let new_cur_msg = string_to_add[index..].to_owned();
            let new_cur_msg_size = new_cur_msg.chars().count();

            self.inner.messages.push(string_to_add[index..].to_owned());
            self.chars_num = new_cur_msg_size;
        } else {
            self.inner.messages.last_mut().unwrap().push_str(string_to_add);
            self.chars_num += string_to_add_size;
        }
        self
    }

    /// Begins a section which affects subsequent calls to `add_string`
    /// 
    /// Does nothing if a section is already in progress
    pub fn begin_section(&mut self) -> &mut Self {
        if self.no_split_section.is_none() {
            self.no_split_section = Some((String::new(), 0));
        }
        self
    }

    #[inline]
    /// Whether we are in a section right now.
    pub fn is_in_section(&self) -> bool {
        self.no_split_section.is_some()
    }

    #[inline]
    /// Ends a section which affects subsequent calls to `add_string`
    /// 
    /// If the section is over the `MSG_LIMIT` it will try to split at a nice point, see `end_section_with`
    /// 
    /// Does nothing if no section is in progress
    pub fn end_section(&mut self) -> &mut Self {
        self.end_section_with(|c| match c {
            ';' | ',' | '.' | '?' | '!' | ')' | ':' | '-' => true,
            _ => false
        })
    }

    /// Ends a section which affects subsequent calls to `add_string`
    /// 
    /// If the section is over the `MSG_LIMIT` it will try to split at a nice point defined by the provided callback.
    /// The callback is used to find characters that are appropriate to split at.
    /// It go travel backwards from the split point, calling the callback until it returns true.
    /// 
    /// Does nothing if no section is in progress
    pub fn end_section_with<F: FnMut(char) -> bool>(&mut self, mut f: F) -> &mut Self {
        if let Some((mut no_split_section, size)) = self.no_split_section.take() {
            if self.chars_num + size > MSG_LIMIT {
                self.chars_num = size;

                let mut no_split_section_size = no_split_section.chars().count();

                // If the section is longer than the msg limit, we have to split it anyway
                // using the passed function to check charactes that should allow splits
                while no_split_section_size > MSG_LIMIT {
                    // take(MSG_LIMIT) so that it'll panic if it doesn't find something to split at before message limit
                    let (mut index, _) = no_split_section.char_indices().rev().skip(no_split_section_size-MSG_LIMIT).take(MSG_LIMIT).find(|(_, c)| f(*c)).unwrap();
                    index += 1;

                    while !no_split_section.is_char_boundary(index) {
                        index += 1;
                    }

                    let new_cur_msg = no_split_section.split_off(index);

                    let first_section = replace(&mut no_split_section, new_cur_msg);
                    no_split_section_size = no_split_section.chars().count();

                    self.inner.messages.push(first_section);
                }
                self.inner.messages.push(no_split_section);
            } else {
                self.chars_num += size;
                self.inner.messages.last_mut().unwrap().push_str(&no_split_section)
            }
        }
        self
    }

    /// Add lines with each line being a separate section
    pub fn add_lines<S: AsRef<str>>(&mut self, lines: S) -> &mut Self {
        for line in lines.as_ref().lines() {
            self.begin_section().add_string(line).add_string("\n").end_section();
        }

        self
    }

    #[inline]
    /// Finalise the current section if one is in progress
    /// and return the final `MsgBunch`
    pub fn build(mut self) -> MsgBunch {
        self.end_section();
        self.inner
    }
}

/// Splits a string into front trim text and end_trim
/// 
/// If the string only consists of whitespace, all but the end trim will be empty.
/// If the string has no whitespace surrounding, the trim strings will be empty.
pub fn split_trim(s: &str) -> (&str, &str, &str) {
    let end_trim_index = s.rfind(|c: char| !c.is_whitespace()).map(|i| {
        i + s[i..].chars().next().unwrap().len_utf8()
    }).unwrap_or(0);
    
    let (start, end_trim) = s.split_at(end_trim_index);
    
    let front_trim_index = start.find(|c: char| !c.is_whitespace()).unwrap_or(end_trim_index);

    let (front_trim, text) = start.split_at(front_trim_index);

    (front_trim, text, end_trim)
}

#[cfg(test)]
mod tests {
    use super::split_trim;
    #[test]
    fn test_split_trim() {
        assert_eq!(split_trim("hestetest"), ("", "hestetest", ""));
        assert_eq!(split_trim("   hest  \n\n asdg \t\n"), ("   ", "hest  \n\n asdg", " \t\n"));
        assert_eq!(split_trim("\n"), ("", "", "\n"));
        assert_eq!(split_trim(" "), ("", "", " "));
    }
}