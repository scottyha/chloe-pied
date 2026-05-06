use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ReviewMode {
    #[default]
    Human,
    Agentic,
}

impl ReviewMode {
    pub const ALL: [Self; 2] = [Self::Human, Self::Agentic];

    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Human => "Human Review",
            Self::Agentic => "Agentic Review",
        }
    }

    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Self::Human => "You review changes manually before committing",
            Self::Agentic => "AI agent reviews and either commits or sends back for changes",
        }
    }
}
