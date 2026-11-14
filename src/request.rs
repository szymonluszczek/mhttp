#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pseudo {
    Method,
    Authority,
    Scheme,
    Path,
}

impl Pseudo {
    pub const CHROME: [Pseudo; 4] = [Self::Method, Self::Authority, Self::Scheme, Self::Path];
    pub const SAFARI: [Pseudo; 4] = [Self::Method, Self::Scheme, Self::Authority, Self::Path];

    pub fn name(self) -> &'static str {
        match self {
            Self::Method => ":method",
            Self::Authority => ":authority",
            Self::Scheme => ":scheme",
            Self::Path => ":path",
        }
    }
}

/// RFC 7540 priority in the HEADERS frame
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Priority {
    pub depends_on: i32,
    /// 1..=256
    pub weight: i32,
    pub exclusive: bool,
}

impl Priority {
    /// exclusive on the root, highest weight
    pub const CHROME: Priority = Priority {
        depends_on: 0,
        weight: 256,
        exclusive: true,
    };

    // negative stream id = no priority
    pub(crate) fn wire(priority: Option<Priority>) -> (i32, i32, i32) {
        match priority {
            Some(p) => (p.depends_on, p.weight, i32::from(p.exclusive)),
            None => (-1, 0, 0),
        }
    }
}

/// where the jar's cookies go; `Off` neither reads the jar nor stores `set-cookie`
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum CookiePlacement {
    #[default]
    Off,
    /// after the pseudo-headers (HTTP/2) or first (HTTP/1.1)
    First,
    Last,
    /// right after this header
    After(String),
}

#[derive(Debug, Clone)]
pub struct Request {
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) body: String,
    pub(crate) cookies: CookiePlacement,
    pub(crate) cookie_order: Vec<String>,
    pub(crate) priority: Option<Option<Priority>>,
}

impl Request {
    pub fn builder(method: &str, path: &str) -> RequestBuilder {
        RequestBuilder {
            request: Request {
                method: method.to_string(),
                path: path.to_string(),
                headers: Vec::new(),
                body: String::new(),
                cookies: CookiePlacement::Off,
                cookie_order: Vec::new(),
                priority: None,
            },
        }
    }

    pub fn get(path: &str) -> RequestBuilder {
        Self::builder("GET", path)
    }

    pub fn post(path: &str) -> RequestBuilder {
        Self::builder("POST", path)
    }

    pub fn method(&self) -> &str {
        &self.method
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    /// as given, without cookies and generated headers
    pub fn headers(&self) -> &[(String, String)] {
        &self.headers
    }

    pub fn body(&self) -> &str {
        &self.body
    }
}

pub struct RequestBuilder {
    request: Request,
}

impl RequestBuilder {
    /// order kept exactly; pseudo-headers not given are generated in the connection's order
    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.request
            .headers
            .push((name.to_string(), value.to_string()));
        self
    }

    pub fn headers<'a>(mut self, headers: impl IntoIterator<Item = (&'a str, &'a str)>) -> Self {
        for (name, value) in headers {
            self.request
                .headers
                .push((name.to_string(), value.to_string()));
        }
        self
    }

    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.request.body = body.into();
        self
    }

    pub fn cookies(mut self, placement: CookiePlacement) -> Self {
        self.request.cookies = placement;
        self
    }

    /// these first, in this order; the rest in jar order
    pub fn cookie_order<'a>(mut self, names: impl IntoIterator<Item = &'a str>) -> Self {
        self.request.cookie_order = names.into_iter().map(str::to_string).collect();
        self
    }

    /// overrides the connection's default; `None` sends none
    pub fn priority(mut self, priority: Option<Priority>) -> Self {
        self.request.priority = Some(priority);
        self
    }

    pub fn build(self) -> Request {
        self.request
    }
}
