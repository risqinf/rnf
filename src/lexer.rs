// ─────────────────────────────────────────────────────────────────────────────
//  RNF Lexer  –  Tokenises .rnf source files
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // ── Literals ──────────────────────────────────────────────────────────────
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),

    // ── Identifiers ───────────────────────────────────────────────────────────
    Ident(String),

    // ── Keywords ──────────────────────────────────────────────────────────────
    Fn,
    Return,
    If,
    Else,
    While,
    Loop,
    In,
    Go,
    Struct,
    Impl,
    Exec,
    Print,
    Let,
    Ptr,
    MakeChan,
    Send,
    Recv,
    RawMem,
    Asm,
    Pub,
    Use,
    Mod,
    Nil,
    Break,
    Continue,

    // ── Operators ─────────────────────────────────────────────────────────────
    Plus,       // +
    Minus,      // -
    Star,       // *
    Slash,      // /
    Percent,    // %
    Assign,     // =
    Eq,         // ==
    Ne,         // !=
    Lt,         // <
    Gt,         // >
    Le,         // <=
    Ge,         // >=
    And,        // &&
    Or,         // ||
    Not,        // !
    Amp,        // &
    Arrow,      // ->
    ChanRecv,   // <-
    DotDot,     // ..

    // ── Delimiters ────────────────────────────────────────────────────────────
    LBrace,     // {
    RBrace,     // }
    LParen,     // (
    RParen,     // )
    LBracket,   // [
    RBracket,   // ]
    Comma,      // ,
    Colon,      // :
    Semicolon,  // ;
    Dot,        // .

    // ── Special ───────────────────────────────────────────────────────────────
    EOF,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub col:  usize,
}

pub struct Lexer {
    src:  Vec<char>,
    pos:  usize,
    line: usize,
    col:  usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Lexer { src: source.chars().collect(), pos: 0, line: 1, col: 1 }
    }

    fn cur(&self) -> Option<char>  { self.src.get(self.pos).copied() }
    fn peek(&self) -> Option<char> { self.src.get(self.pos + 1).copied() }

    fn advance(&mut self) -> Option<char> {
        let c = self.cur();
        if let Some(ch) = c {
            self.pos += 1;
            if ch == '\n' { self.line += 1; self.col = 1; }
            else           { self.col  += 1; }
        }
        c
    }

    fn can_end_stmt(kind: &TokenKind) -> bool {
        matches!(kind,
            TokenKind::Ident(_)
            | TokenKind::Int(_)
            | TokenKind::Float(_)
            | TokenKind::Str(_)
            | TokenKind::Bool(_)
            | TokenKind::Nil
            | TokenKind::RParen
            | TokenKind::RBracket
            | TokenKind::RBrace
            | TokenKind::Return
            | TokenKind::Break
            | TokenKind::Continue
        )
    }

    fn skip_ws_comments(&mut self, last: &Option<TokenKind>) -> bool {
        let mut inserted = false;
        loop {
            match self.cur() {
                Some('\n') => {
                    self.advance();
                    if let Some(lk) = last {
                        if Self::can_end_stmt(lk) { inserted = true; }
                    }
                }
                Some(' ') | Some('\t') | Some('\r') => { self.advance(); }
                Some('/') if self.peek() == Some('/') => {
                    while self.cur() != Some('\n') && self.cur().is_some() { self.advance(); }
                }
                Some('/') if self.peek() == Some('*') => {
                    self.advance(); self.advance();
                    loop {
                        if self.cur() == Some('*') && self.peek() == Some('/') {
                            self.advance(); self.advance(); break;
                        }
                        if self.cur().is_none() { break; }
                        self.advance();
                    }
                }
                _ => break,
            }
        }
        inserted
    }

    fn read_string(&mut self) -> Result<TokenKind, String> {
        self.advance(); // consume opening "
        let mut s = String::new();
        loop {
            match self.cur() {
                Some('"')  => { self.advance(); return Ok(TokenKind::Str(s)); }
                Some('\\') => {
                    self.advance();
                    match self.advance() {
                        Some('n')  => s.push('\n'),
                        Some('t')  => s.push('\t'),
                        Some('r')  => s.push('\r'),
                        Some('"')  => s.push('"'),
                        Some('\\') => s.push('\\'),
                        Some('0')  => s.push('\0'),
                        Some(c)    => { s.push('\\'); s.push(c); }
                        None => return Err("Unterminated string escape".into()),
                    }
                }
                Some(c) => { s.push(c); self.advance(); }
                None => return Err(format!("Unterminated string at line {}", self.line)),
            }
        }
    }

    fn read_number(&mut self) -> TokenKind {
        let mut s = String::new();
        let mut is_float = false;

        // Hex literals
        if self.cur() == Some('0') && (self.peek() == Some('x') || self.peek() == Some('X')) {
            self.advance(); self.advance();
            let mut hex = String::new();
            while let Some(c) = self.cur() {
                if c.is_ascii_hexdigit() { hex.push(c); self.advance(); } else { break; }
            }
            return TokenKind::Int(i64::from_str_radix(&hex, 16).unwrap_or(0));
        }

        while let Some(c) = self.cur() {
            if c.is_ascii_digit() {
                s.push(c); self.advance();
            } else if c == '.' && !is_float && self.peek().map_or(false, |p| p.is_ascii_digit()) {
                is_float = true; s.push(c); self.advance();
            } else { break; }
        }

        if is_float { TokenKind::Float(s.parse().unwrap_or(0.0)) }
        else        { TokenKind::Int(s.parse().unwrap_or(0)) }
    }

    fn read_ident_or_kw(&mut self) -> TokenKind {
        let mut s = String::new();
        while let Some(c) = self.cur() {
            if c.is_alphanumeric() || c == '_' { s.push(c); self.advance(); } else { break; }
        }
        match s.as_str() {
            "fn"        => TokenKind::Fn,
            "return"    => TokenKind::Return,
            "if"        => TokenKind::If,
            "else"      => TokenKind::Else,
            "while"     => TokenKind::While,
            "loop"      => TokenKind::Loop,
            "in"        => TokenKind::In,
            "go"        => TokenKind::Go,
            "struct"    => TokenKind::Struct,
            "impl"      => TokenKind::Impl,
            "exec"      => TokenKind::Exec,
            "print"     => TokenKind::Print,
            "let"       => TokenKind::Let,
            "ptr"       => TokenKind::Ptr,
            "make_chan" => TokenKind::MakeChan,
            "send"      => TokenKind::Send,
            "recv"      => TokenKind::Recv,
            "raw_mem"   => TokenKind::RawMem,
            "asm"       => TokenKind::Asm,
            "pub"       => TokenKind::Pub,
            "use"       => TokenKind::Use,
            "mod"       => TokenKind::Mod,
            "nil"       => TokenKind::Nil,
            "break"     => TokenKind::Break,
            "continue"  => TokenKind::Continue,
            "true"      => TokenKind::Bool(true),
            "false"     => TokenKind::Bool(false),
            _           => TokenKind::Ident(s),
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();
        let mut last_kind: Option<TokenKind> = None;

        loop {
            // ASI: insert implicit semicolon if last token can end a statement and we crossed a newline
            let inserted = self.skip_ws_comments(&last_kind);
            if inserted {
                let line = self.line; let col = self.col;
                tokens.push(Token { kind: TokenKind::Semicolon, line, col });
                last_kind = Some(TokenKind::Semicolon);
                continue;
            }

            let line = self.line;
            let col  = self.col;

            let kind = match self.cur() {
                None       => { tokens.push(Token { kind: TokenKind::EOF, line, col }); break; }
                Some('"')  => self.read_string()?,
                Some(c) if c.is_ascii_digit() => self.read_number(),
                Some(c) if c.is_alphabetic() || c == '_' => self.read_ident_or_kw(),

                Some('+') => { self.advance(); TokenKind::Plus }
                Some('-') => {
                    self.advance();
                    if self.cur() == Some('>') { self.advance(); TokenKind::Arrow }
                    else { TokenKind::Minus }
                }
                Some('*') => { self.advance(); TokenKind::Star }
                Some('/') => { self.advance(); TokenKind::Slash }
                Some('%') => { self.advance(); TokenKind::Percent }
                Some('=') => {
                    self.advance();
                    if self.cur() == Some('=') { self.advance(); TokenKind::Eq }
                    else { TokenKind::Assign }
                }
                Some('!') => {
                    self.advance();
                    if self.cur() == Some('=') { self.advance(); TokenKind::Ne }
                    else { TokenKind::Not }
                }
                Some('<') => {
                    self.advance();
                    if self.cur() == Some('=') { self.advance(); TokenKind::Le }
                    else if self.cur() == Some('-') { self.advance(); TokenKind::ChanRecv }
                    else { TokenKind::Lt }
                }
                Some('>') => {
                    self.advance();
                    if self.cur() == Some('=') { self.advance(); TokenKind::Ge }
                    else { TokenKind::Gt }
                }
                Some('&') => {
                    self.advance();
                    if self.cur() == Some('&') { self.advance(); TokenKind::And }
                    else { TokenKind::Amp }
                }
                Some('|') => {
                    self.advance();
                    if self.cur() == Some('|') { self.advance(); TokenKind::Or }
                    else { return Err(format!("Unexpected '|' at {}:{} — did you mean '||'?", line, col)); }
                }
                Some('.') => {
                    self.advance();
                    if self.cur() == Some('.') { self.advance(); TokenKind::DotDot }
                    else { TokenKind::Dot }
                }
                Some('{') => { self.advance(); TokenKind::LBrace }
                Some('}') => { self.advance(); TokenKind::RBrace }
                Some('(') => { self.advance(); TokenKind::LParen }
                Some(')') => { self.advance(); TokenKind::RParen }
                Some('[') => { self.advance(); TokenKind::LBracket }
                Some(']') => { self.advance(); TokenKind::RBracket }
                Some(',') => { self.advance(); TokenKind::Comma }
                Some(':') => { self.advance(); TokenKind::Colon }
                Some(';') => { self.advance(); TokenKind::Semicolon }
                Some(c)   => return Err(format!("Unexpected character '{}' at {}:{}", c, line, col)),
            };

            tokens.push(Token { kind: kind.clone(), line, col });
            last_kind = Some(kind);
        }

        Ok(tokens)
    }
}
