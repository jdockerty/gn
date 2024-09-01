use std::fmt::Display;

use clap::ValueEnum;

#[derive(Default, Clone, ValueEnum)]
pub enum Protocol {
    #[default]
    Tcp,
    Udp,
}

impl From<&str> for Protocol {
    fn from(value: &str) -> Self {
        match value {
            "tcp" | "TCP" => Self::Tcp,
            "udp" | "UDP" => Self::Udp,
            _ => panic!("unsupported connection type: {value}"),
        }
    }
}

impl Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tcp => write!(f, "tcp"),
            Self::Udp => write!(f, "udp"),
        }
    }
}
