use iso8601_timestamp::Timestamp;

auto_derived!(
    /// Platform policy change
    pub struct PolicyChange {
        /// Time at which this policy was created
        pub created_time: Timestamp,
        /// Time at which this policy is effective
        pub effective_time: Timestamp,

        /// Message shown to users
        pub description: String,
        /// URL with details about changes
        pub url: String,
    }
);
