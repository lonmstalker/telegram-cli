use std::{error::Error, fmt, str::FromStr};

/// Стабильный идентификатор feature contract из `HARNESS.md`.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum FeatureId {
    F001,
    F002,
    F003,
    F004,
    F005,
    F006,
    F007,
    F008,
    F009,
    F010,
    F011,
    F012,
    F013,
    F014,
    F015,
    F016,
    F017,
    F018,
    F019,
    F020,
    F021,
    F022,
}

impl FeatureId {
    pub const ALL: [Self; 22] = [
        Self::F001,
        Self::F002,
        Self::F003,
        Self::F004,
        Self::F005,
        Self::F006,
        Self::F007,
        Self::F008,
        Self::F009,
        Self::F010,
        Self::F011,
        Self::F012,
        Self::F013,
        Self::F014,
        Self::F015,
        Self::F016,
        Self::F017,
        Self::F018,
        Self::F019,
        Self::F020,
        Self::F021,
        Self::F022,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::F001 => "F001",
            Self::F002 => "F002",
            Self::F003 => "F003",
            Self::F004 => "F004",
            Self::F005 => "F005",
            Self::F006 => "F006",
            Self::F007 => "F007",
            Self::F008 => "F008",
            Self::F009 => "F009",
            Self::F010 => "F010",
            Self::F011 => "F011",
            Self::F012 => "F012",
            Self::F013 => "F013",
            Self::F014 => "F014",
            Self::F015 => "F015",
            Self::F016 => "F016",
            Self::F017 => "F017",
            Self::F018 => "F018",
            Self::F019 => "F019",
            Self::F020 => "F020",
            Self::F021 => "F021",
            Self::F022 => "F022",
        }
    }
}

impl fmt::Display for FeatureId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl TryFrom<&str> for FeatureId {
    type Error = ParseFeatureIdError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "F001" => Ok(Self::F001),
            "F002" => Ok(Self::F002),
            "F003" => Ok(Self::F003),
            "F004" => Ok(Self::F004),
            "F005" => Ok(Self::F005),
            "F006" => Ok(Self::F006),
            "F007" => Ok(Self::F007),
            "F008" => Ok(Self::F008),
            "F009" => Ok(Self::F009),
            "F010" => Ok(Self::F010),
            "F011" => Ok(Self::F011),
            "F012" => Ok(Self::F012),
            "F013" => Ok(Self::F013),
            "F014" => Ok(Self::F014),
            "F015" => Ok(Self::F015),
            "F016" => Ok(Self::F016),
            "F017" => Ok(Self::F017),
            "F018" => Ok(Self::F018),
            "F019" => Ok(Self::F019),
            "F020" => Ok(Self::F020),
            "F021" => Ok(Self::F021),
            "F022" => Ok(Self::F022),
            _ => Err(ParseFeatureIdError),
        }
    }
}

impl FromStr for FeatureId {
    type Err = ParseFeatureIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::try_from(value)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParseFeatureIdError;

impl fmt::Display for ParseFeatureIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ожидался feature ID F001..F022")
    }
}

impl Error for ParseFeatureIdError {}

#[cfg(test)]
mod tests;
