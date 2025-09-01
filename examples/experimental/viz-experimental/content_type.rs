//todo
//Header 只需要传一个值("Content-Type": "xx") //参考了 Actix
use mime::Mime;
use std::fmt;

// 自定义 ContentType 类型
#[derive(Debug, Clone)]
pub struct ContentType(Mime);

impl ContentType {
    // 快捷方法创建特定 MIME 类型
    pub fn json() -> Self {
        ContentType(mime::APPLICATION_JSON)
    }

    pub fn html() -> Self {
        ContentType(mime::TEXT_HTML)
    }

    pub fn text() -> Self {
        ContentType(mime::TEXT_PLAIN)
    }

    pub fn form() -> Self {
        ContentType(mime::APPLICATION_WWW_FORM_URLENCODED)
    }

    pub fn multipart() -> Self {
        ContentType(mime::MULTIPART_FORM_DATA)
    }

    pub fn xml() -> Self {
        ContentType(mime::TEXT_XML)
    }

    pub fn javascript() -> Self {
        ContentType(mime::APPLICATION_JAVASCRIPT)
    }

    pub fn css() -> Self {
        ContentType(mime::TEXT_CSS)
    }

    pub fn pdf() -> Self {
        ContentType(mime::APPLICATION_PDF)
    }

    pub fn png() -> Self {
        ContentType(mime::IMAGE_PNG)
    }

    pub fn jpeg() -> Self {
        ContentType(mime::IMAGE_JPEG)
    }

    pub fn gif() -> Self {
        ContentType(mime::IMAGE_GIF)
    }

    pub fn svg() -> Self {
        ContentType(mime::IMAGE_SVG)
    }

    pub fn octet_stream() -> Self {
        ContentType(mime::APPLICATION_OCTET_STREAM)
    }

    // 获取 MIME 类型字符串
    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }

    // 转换为 HTTP 头部
    pub fn to_header(&self) -> Header {
        Header::new("Content-Type", self.0.to_string())
    }
}

// 实现 Display 以便直接打印
impl fmt::Display for ContentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", "Content-Type", self.0)
    }
}

// 自定义头部键值对类型
#[derive(Debug, Clone)]
pub struct Header {
    pub name: &'static str,
    pub value: String,
}

impl Header {
    pub fn new<T: Into<String>>(name: &'static str, value: T) -> Self {
        Self { name, value: value.into() }
    }
}

// 实现 Display trait 以便直接打印
impl fmt::Display for Header {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.value)
    }
}

macro_rules! standard_headers {
    ($(($struct_name:ident, $const_name:ident, $str_const:ident, $header_str:expr));* $(;)?) => {
        $(
            #[derive(Debug, Clone)]
            pub struct $struct_name;

            impl $struct_name {
                // 字符串版本
                pub const fn name() -> &'static str {
                    $header_str
                }

                // 字节版本
                pub const fn name_bytes() -> &'static [u8] {
                    $header_str.as_bytes()
                }

                // 字符串版本的 with_value
                pub fn with_value<T: Into<String>>(value: T) -> Header {
                    Header::new(Self::name(), value.into())
                }

                // 字节版本的 with_value
                pub fn with_value_bytes<T: Into<String>>(value: T) -> (Vec<u8>, String) {
                    (Self::name_bytes().to_vec(), value.into())
                }
            }

            // 字符串常量
            pub const $const_name: &str = $header_str;
            // 字节常量
            pub const $str_const: &[u8] = $header_str.as_bytes();
        )*
    };
}

// 为 Authorization 添加常用方法
impl Authorization {
    pub fn bearer<T: Into<String>>(token: T) -> Header {
        Header::new("authorization", format!("Bearer {}", token.into()))
    }

    pub fn basic<T: Into<String>>(credentials: T) -> Header {
        Header::new("authorization", format!("Basic {}", credentials.into()))
    }
}

standard_headers! {
    (Accept, ACCEPT, ACCEPT_BYTES, "accept");
    (AcceptCharset, ACCEPT_CHARSET, ACCEPT_CHARSET_BYTES, "accept-charset");
    (AcceptEncoding, ACCEPT_ENCODING, ACCEPT_ENCODING_BYTES, "accept-encoding");
    (Authorization, AUTHORIZATION, AUTHORIZATION_BYTES, "authorization");
    (AcceptLanguage, ACCEPT_LANGUAGE, ACCEPT_LANGUAGE_BYTES, "accept-language");
    (AcceptRanges, ACCEPT_RANGES, ACCEPT_RANGES_BYTES, "accept-ranges");
    (ContentTypeHeader, CONTENT_TYPE, CONTENT_TYPE_BYTES, "content-type");
}
