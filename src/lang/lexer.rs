use super::{ Symbol, Error, ErrorKind, LocationArea, LocationPoint };

pub struct Lexer<'a, I: Iterator<Item = char>> {
	src: std::iter::Peekable<&'a mut I>,
	location: LocationPoint,
	current: LocationPoint,
}

impl<'a, I: Iterator<Item = char>> Lexer<'a, I> {
	pub fn new(src: &'a mut I) -> Self {
		Self {
			src: src.peekable(),
			location: LocationPoint::default(),
			current: LocationPoint::default(),
		}
	}

	fn eat(&mut self) -> Option<char> {
		let ch = self.src.next()?;

		self.current = self.location;
		self.location.index += 1;

		if ch == '\n' {
			self.location.line += 1;
			self.location.column = 0;
		} else {
			self.location.column += 1;
		}

		Some(ch)
	}

	fn eat_n(&mut self, n: usize) -> Option<String> {
		let mut string = String::new();

		for _ in 0..n {
			string.push(self.eat()?);
		}

		Some(string)
	}
}

impl<'a, I: Iterator<Item = char>> Iterator for Lexer<'a, I> {
	type Item = Result<Token, Error>;

	fn next(&mut self) -> Option<Self::Item> {
		while let Some(ch) = self.src.peek() {
			if ch.is_ascii_whitespace() {
				self.eat();
			} else {
				break
			}
		}

		let ch = self.eat()?;
		let start = self.current;

		let kind = match ch {
			'(' => TokenKind::LeftParen,
			')' => TokenKind::RightParen,
			'[' => TokenKind::LeftBracket,
			']' => TokenKind::RightBracket,
			'{' => TokenKind::LeftBrace,
			'}' => TokenKind::RightBrace,
			'.' => TokenKind::Dot,

			'A'..='Z' | 'a'..='z' | '_' => {
				let mut symbol = String::from(ch);
				while let Some('0'..='9' | 'A'..='Z' | 'a'..='z' | '_') = self.src.peek() {
					symbol.push(self.eat().unwrap());
				}

				match symbol.as_str() {
					"true" => TokenKind::Boolean(true),
					"false" => TokenKind::Boolean(false),
					"nil" => TokenKind::Nil,

					_ => TokenKind::Symbol(Symbol::new(symbol).unwrap()),
				}
			},

			'-' | '+' | '0'..='9' => {
				let mut number = String::from(ch);
				while let Some('0'..='9' | '_' | '.' | 'E' | 'e') = self.src.peek() {
					number.push(self.eat().unwrap());
				}

				use std::str::FromStr;
				TokenKind::Number(match f32::from_str(&number) {
					Ok(number) => number,
					Err(_) => return Some(Err(Error {
						kind: ErrorKind::SyntaxError,
						location: Some(LocationArea { start, end: self.current }),
						message: "invalid number literal".into(),
					})),
				})
			},
			'"' | '\'' => {
				let mut string = String::new();
				loop {
					if let Some(nch) = self.eat() {
						if nch == ch {
							break
						} else if nch == '\\' {
							let before = self.current;
							if let Some(ech) = self.eat() {
								string.push(match ech {
									'x' => match self.eat_n(2) {
										Some(hex) => match u8::from_str_radix(&hex, 16) {
											Ok(octet) => octet as char,
											Err(_) => return Some(Err(Error {
												kind: ErrorKind::SyntaxError,
												location: Some(LocationArea {
													start: before,
													end: self.current,
												}),
												message: format!("{:?} is invalid hex", hex),
											})),
										},
										None => return Some(Err(Error {
											kind: ErrorKind::SyntaxError,
											location: Some(self.current.into()),
											message: "unexpected end whilst parsing escape".into(),
										})),
									},
									'u' => match self.eat_n(4) {
										Some(hex) => match u16::from_str_radix(&hex, 16) {
											Ok(word) => match char::from_u32(word as u32) {
												Some(uni) => uni,
												None => return Some(Err(Error {
													kind: ErrorKind::SyntaxError,
													location: Some(LocationArea {
														start: before,
														end: self.current,
													}),
													message: format!("{} is not a valid character", word),
												})),
											},
											Err(_) => return Some(Err(Error {
												kind: ErrorKind::SyntaxError,
												location: Some(LocationArea {
													start: before,
													end: self.current,
												}),
												message: format!("{:?} is invalid hex", hex),
											})),
										},
										None => return Some(Err(Error {
											kind: ErrorKind::SyntaxError,
											location: Some(self.current.into()),
											message: "unexpected end whilst parsing escape".into(),
										})),
									},
									'U' =>  match self.eat_n(8) {
										Some(hex) => match u32::from_str_radix(&hex, 16) {
											Ok(dword) => match char::from_u32(dword) {
												Some(uni) => uni,
												None => return Some(Err(Error {
													kind: ErrorKind::SyntaxError,
													location: Some(LocationArea {
														start: before,
														end: self.current,
													}),
													message: format!("{} is not a valid character", dword),
												})),
											},
											Err(_) => return Some(Err(Error {
												kind: ErrorKind::SyntaxError,
												location: Some(LocationArea {
													start: before,
													end: self.current,
												}),
												message: format!("{:?} is invalid hex", hex),
											})),
										},
										None => return Some(Err(Error {
											kind: ErrorKind::SyntaxError,
											location: Some(self.current.into()),
											message: "unexpected end whilst parsing escape".into(),
										})),
									},

									'n' => '\n',
									'r' => '\r',
									't' => '\t',

									'0' => '\0',
									'\\' => '\\',

									_ => return Some(Err(Error {
										kind: ErrorKind::SyntaxError,
										location: Some(LocationArea {
											start: before,
											end: self.current,
										}),
										message: format!("{:?} is not a valid escape", ech),
									})),
								});
							} else {
								return Some(Err(Error {
									kind: ErrorKind::SyntaxError,
									location: Some(self.current.into()),
									message: "unexpected end whilst parsing escape".into(),
								}))
							}
						} else {
							string.push(nch);
						}
					} else {
						return Some(Err(Error {
							kind: ErrorKind::SyntaxError,
							location: Some(self.current.into()),
							message: "unterminated string".into(),
						}))
					}
				}

				TokenKind::String(string)
			},

			ch => {
				if ch == '/' {
					if let Some('/' | '*') = self.src.peek() {
						if self.eat().unwrap() == '/' {
							loop {
								match self.eat() {
									Some('\n') => break,
									Some(_) => continue,
									None => return None,
								}
							}
						} else {
							let mut expect_end = false;
							loop {
								match self.eat() {
									Some('*') => { expect_end = true; continue },
									Some('/') => if expect_end { break },
									Some(_) => (),
									None => return Some(Err(Error {
										kind: ErrorKind::SyntaxError,
										location: Some(self.current.into()),
										message: "unterminated comment".into(),
									})),
								}

								expect_end = false;
							}
						}

						// this is stupid
						return self.next()
					}
				}

				return Some(Err(Error {
					kind: ErrorKind::SyntaxError,
					location: Some(self.current.into()),
					message: format!("unexpected {:?}", ch),
				}))
			},
		};

		if
			kind != TokenKind::LeftParen &&
			kind != TokenKind::LeftBracket &&
			kind != TokenKind::LeftBrace &&
			kind != TokenKind::Dot
		{
			if let Some(ch) = self.src.peek() {
				let exempt = if let TokenKind::Symbol(_) = kind {
					*ch == '.'
				} else {
					false
				};

				if !exempt && !ch.is_ascii_whitespace() {
					if let ')' | ']' | '}' = ch {} else {
						return Some(Err(Error {
							kind: ErrorKind::SyntaxError,
							location: Some(self.location.into()),
							message: "expected delimeter".into(),
						}))
					}
				}
			}
		}

		Some(Ok(Token {
			kind,
			location: LocationArea { start, end: self.current },
		}))
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
	pub(super) kind: TokenKind,
	pub location: LocationArea,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
	LeftParen,
	RightParen,
	LeftBracket,
	RightBracket,
	LeftBrace,
	RightBrace,
	Dot,

	Symbol(Symbol),

	Number(f32),
	String(String),
	Boolean(bool),
	Nil,
}
