use std::collections::{HashSet, VecDeque};

use logos::Logos;

#[derive(Debug, Clone, Logos, PartialEq)]
#[logos(skip "\n")]
#[logos(subpattern id="[0123456789ABCDEFGHJKMNPQRSTVWXYZ]{26}")]
pub enum MessageToken<'a> {
    #[token("\\")]
    Escape,
    #[regex("```[^`\n]*", |_| 3)]
    #[regex("``", |_| 2)]
    #[regex("`", |_| 1)]
    CodeblockMarker(usize),
    #[regex("<@(?&id)>", |lex| &lex.slice()[2..lex.slice().len() - 1])]
    UserMention(&'a str),
    #[regex("<%(?&id)>", |lex| &lex.slice()[2..lex.slice().len() - 1],)]
    RoleMention(&'a str),
    #[token("@everyone")]
    MentionEveryone,
    #[token("@online")]
    MentionOnline
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MessageResults {
    pub user_mentions: HashSet<String>,
    pub role_mentions: HashSet<String>,
    pub mentions_everyone: bool,
    pub mentions_online: bool
}

struct MessageParserIterator<'a, I> {
    inner: I,
    temp: VecDeque<MessageToken<'a>>
}

impl<'a, I: Iterator<Item = MessageToken<'a>>> Iterator for MessageParserIterator<'a, I> {
    type Item = MessageToken<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.temp.is_empty() {
            self.temp.pop_front()
        } else {
            let token = self.inner.next();

            if token == Some(MessageToken::Escape) {
                self.inner.next();

                token
            } else if let Some(MessageToken::CodeblockMarker(ty)) = token {
                loop {
                    let next_token = self.inner.next();

                    if next_token == Some(MessageToken::CodeblockMarker(ty)) {
                        self.temp.clear();
                        self.temp.push_back(MessageToken::CodeblockMarker(ty));
                        break next_token
                    } else if let Some(token) = next_token {
                        self.temp.push_back(token);
                    } else {
                        break Some(MessageToken::CodeblockMarker(ty))
                    }
                }
            } else {
                token
            }
        }
    }
}

pub fn parse_message_iter(text: &str) -> impl Iterator<Item = MessageToken<'_>> + '_ {
    MessageParserIterator {
        inner: MessageToken::lexer(text).flatten(),
        temp: VecDeque::new()
    }
}

pub fn parse_message(text: &str) -> MessageResults {
    let mut results = MessageResults::default();

    for token in parse_message_iter(text) {
        match token {
            MessageToken::Escape => {}
            MessageToken::CodeblockMarker(_) => {},
            MessageToken::UserMention(id) => { results.user_mentions.insert(id.to_string()); },
            MessageToken::RoleMention(id) => { results.role_mentions.insert(id.to_string()); },
            MessageToken::MentionEveryone => results.mentions_everyone = true,
            MessageToken::MentionOnline => results.mentions_online = true,
        };
    };

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_nodes() {
        let output = parse_message_iter("Hello everyone").collect::<Vec<_>>();

        assert_eq!(output.len(), 0);
    }

    #[test]
    fn test_simple_user_mention() {
        let output = parse_message_iter("Hello <@01FD58YK5W7QRV5H3D64KTQYX3>.").collect::<Vec<_>>();

        assert_eq!(output.len(), 1);
        assert_eq!(output[0], MessageToken::UserMention("01FD58YK5W7QRV5H3D64KTQYX3"));
    }

    #[test]
    fn test_simple_role_mention() {
        let output = parse_message_iter("Hello <%01FD58YK5W7QRV5H3D64KTQYX3>.").collect::<Vec<_>>();

        assert_eq!(output.len(), 1);
        assert_eq!(output[0], MessageToken::RoleMention("01FD58YK5W7QRV5H3D64KTQYX3"));
    }

    #[test]
    fn test_mention_everyone() {
        let output = parse_message_iter("Hello @everyone.").collect::<Vec<_>>();

        assert_eq!(output.len(), 1);
        assert_eq!(output[0], MessageToken::MentionEveryone);
    }

    #[test]
    fn test_mention_online() {
        let output = parse_message_iter("Hello @online.").collect::<Vec<_>>();

        assert_eq!(output.len(), 1);
        assert_eq!(output[0], MessageToken::MentionOnline);
    }

    #[test]
    fn test_everything() {
        let output = parse_message_iter("Hello <@01FD58YK5W7QRV5H3D64KTQYX3>, <%01FD58YK5W7QRV5H3D64KTQYX3>, @everyone and @online.").collect::<Vec<_>>();

        assert_eq!(output.len(), 4);
        assert_eq!(output[0], MessageToken::UserMention("01FD58YK5W7QRV5H3D64KTQYX3"));
        assert_eq!(output[1], MessageToken::RoleMention("01FD58YK5W7QRV5H3D64KTQYX3"));
        assert_eq!(output[2], MessageToken::MentionEveryone);
        assert_eq!(output[3], MessageToken::MentionOnline);
    }

    #[test]
    fn test_everything_no_spaces() {
        let output = parse_message_iter("<@01FD58YK5W7QRV5H3D64KTQYX3><%01FD58YK5W7QRV5H3D64KTQYX3>@everyone@online").collect::<Vec<_>>();

        assert_eq!(output.len(), 4);
        assert_eq!(output[0], MessageToken::UserMention("01FD58YK5W7QRV5H3D64KTQYX3"));
        assert_eq!(output[1], MessageToken::RoleMention("01FD58YK5W7QRV5H3D64KTQYX3"));
        assert_eq!(output[2], MessageToken::MentionEveryone);
        assert_eq!(output[3], MessageToken::MentionOnline);
    }

    #[test]
    fn test_codeblock_no_mentions() {
        let output = parse_message_iter("```\n<@01FD58YK5W7QRV5H3D64KTQYX3><%01FD58YK5W7QRV5H3D64KTQYX3>@everyone@online\n```").collect::<Vec<_>>();

        assert_eq!(output.len(), 2);
        assert_eq!(output[0], MessageToken::CodeblockMarker(3));
        assert_eq!(output[1], MessageToken::CodeblockMarker(3));
    }

    #[test]
    fn test_uncontained_codeblock_should_mention() {
        let output = parse_message_iter("```\n<@01FD58YK5W7QRV5H3D64KTQYX3><%01FD58YK5W7QRV5H3D64KTQYX3>@everyone@online").collect::<Vec<_>>();

        assert_eq!(output.len(), 5);
        assert_eq!(output[0], MessageToken::CodeblockMarker(3));
        assert_eq!(output[1], MessageToken::UserMention("01FD58YK5W7QRV5H3D64KTQYX3"));
        assert_eq!(output[2], MessageToken::RoleMention("01FD58YK5W7QRV5H3D64KTQYX3"));
        assert_eq!(output[3], MessageToken::MentionEveryone);
        assert_eq!(output[4], MessageToken::MentionOnline);
    }

    #[test]
    fn test_inline_codeblock_no_mentions() {
        let output = parse_message_iter("`<@01FD58YK5W7QRV5H3D64KTQYX3><%01FD58YK5W7QRV5H3D64KTQYX3>@everyone@online`").collect::<Vec<_>>();

        assert_eq!(output.len(), 2);
        assert_eq!(output[0], MessageToken::CodeblockMarker(1));
        assert_eq!(output[0], MessageToken::CodeblockMarker(1));
    }

    #[test]
    fn test_uncontained_inline_codeblock_should_mention() {
        let output = parse_message_iter("`<@01FD58YK5W7QRV5H3D64KTQYX3><%01FD58YK5W7QRV5H3D64KTQYX3>@everyone@online").collect::<Vec<_>>();

        assert_eq!(output.len(), 5);
        assert_eq!(output[0], MessageToken::CodeblockMarker(1));
        assert_eq!(output[1], MessageToken::UserMention("01FD58YK5W7QRV5H3D64KTQYX3"));
        assert_eq!(output[2], MessageToken::RoleMention("01FD58YK5W7QRV5H3D64KTQYX3"));
        assert_eq!(output[3], MessageToken::MentionEveryone);
        assert_eq!(output[4], MessageToken::MentionOnline);
    }

    #[test]
    fn test_codeblock_with_language_no_mentions() {
        let output = parse_message_iter("```rust\n<@01FD58YK5W7QRV5H3D64KTQYX3><%01FD58YK5W7QRV5H3D64KTQYX3>@everyone@online```").collect::<Vec<_>>();

        assert_eq!(output.len(), 2);
        assert_eq!(output[0], MessageToken::CodeblockMarker(3));
        assert_eq!(output[1], MessageToken::CodeblockMarker(3));
    }

    #[test]
    fn test_double_inline_codeblock() {
        let output = parse_message_iter("``this should not ping @everyone``").collect::<Vec<_>>();

        assert_eq!(output.len(), 2);
        assert_eq!(output[0], MessageToken::CodeblockMarker(2));
        assert_eq!(output[1], MessageToken::CodeblockMarker(2));
    }

    #[test]
    fn test_double_inline_codeblock_with_backticks_inside() {
        let output = parse_message_iter("``this `should` not `ping` @everyone``").collect::<Vec<_>>();

        assert_eq!(output.len(), 2);
        assert_eq!(output[0], MessageToken::CodeblockMarker(2));
        assert_eq!(output[1], MessageToken::CodeblockMarker(2));
    }

    #[test]
    fn test_in_middle() {
        let output = parse_message_iter("i am not pinging `@everyone`.").collect::<Vec<_>>();

        assert_eq!(output.len(), 2);
        assert_eq!(output[0], MessageToken::CodeblockMarker(1));
        assert_eq!(output[1], MessageToken::CodeblockMarker(1));
    }

    #[test]
    fn test_escaped_codeblock() {
        let output = parse_message_iter("i am ~~not~~ pinging \\`@everyone` ok.").collect::<Vec<_>>();

        assert_eq!(output.len(), 3);
        assert_eq!(output[0], MessageToken::Escape);
        assert_eq!(output[1], MessageToken::MentionEveryone);
        assert_eq!(output[2], MessageToken::CodeblockMarker(1));
    }

    #[test]
    fn test_escape_mention() {
        let output = parse_message_iter("i wont ping \\@everyone").collect::<Vec<_>>();

        assert_eq!(output.len(), 1);
        assert_eq!(output[0], MessageToken::Escape);
    }
}
