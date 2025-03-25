use std::str::FromStr;

pub(super) struct DashUrl {
    pub(super) adaptation_set: usize,
    pub(super) period: usize,
    pub(super) representation: usize,
}

impl DashUrl {
    pub(super) fn new(period: usize, adaptation_set: usize, representation: usize) -> Self {
        Self {
            adaptation_set,
            period,
            representation,
        }
    }
}

impl std::fmt::Display for DashUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "dash://period.{}.adaptation-set.{}.representation.{}",
            self.period, self.adaptation_set, self.representation
        )
    }
}

impl FromStr for DashUrl {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        if !s.starts_with("dash://") {
            return Err(format!(
                "url doesn't have dash scheme \
            (expected: dash://period.{{}}.adaptation-set.{{}}.representation.{{}}, found: {})",
                s
            ));
        }

        let location = s
            .replace("dash://", "")
            .replace("period", "")
            .replace("adaptation-set", "")
            .replace("representation", "")
            .split_terminator('.')
            .filter_map(|x| x.parse::<usize>().ok())
            .collect::<Vec<usize>>();

        if location.len() != 3 {
            return Err(format!(
                "url doesn't have full location to locate dash resource \
            (expected: dash://period.{{}}.adaptation-set.{{}}.representation.{{}}, found: {})",
                s
            ));
        }

        Ok(Self {
            adaptation_set: location[1],
            period: location[0],
            representation: location[2],
        })
    }
}
