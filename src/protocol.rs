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
