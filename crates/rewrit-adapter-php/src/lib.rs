//! PHP adapter boundary for Pest, PHPUnit and Laravel helpers.

#![forbid(unsafe_code)]

pub const PEST_ADAPTER: &str = "php:pest";
pub const PHPUNIT_ADAPTER: &str = "php:phpunit";
pub const LARAVEL_ADAPTER: &str = "php:laravel";

pub mod laravel {
    pub const SDK_HELPER: &str = "Rewrit\\Rewrit";
}

pub mod pest {
    pub const METHOD: &str = "rewrit";
}

pub mod phpunit {
    pub const EXTENSION: &str = "Rewrit\\PHPUnitExtension";
    pub const CASE_TRAIT: &str = "Rewrit\\PHPUnitCase";
}
