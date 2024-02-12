use derivative::Derivative;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DisplayFromStr;

#[serde_as]
#[derive(Clone, Deserialize, Serialize, Derivative)]
#[derivative(Debug)]
pub struct AVMConfig {
    /// Maximum heap size in bytes available for an interpreter instance.
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(default)]
    pub aquavm_heap_size_limit: Option<bytesize::ByteSize>,

    /// Maximum AIR size in bytes that is used by the AquaVM limit check.
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(default)]
    pub air_size_limit: Option<bytesize::ByteSize>,

    /// Maximum particle size in bytes that is used by the AquaVM limit check.
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(default)]
    pub particle_size_limit: Option<bytesize::ByteSize>,

    /// Maximum service call result size in bytes that is used by the AquaVM limit check.
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(default)]
    pub call_result_size_limit: Option<bytesize::ByteSize>,

    /// Hard limit AquaVM behavior control knob.
    #[serde(default)]
    pub hard_limit_enabled: bool,
}

impl Default for AVMConfig {
    fn default() -> Self {
        Self {
            aquavm_heap_size_limit: None,
            air_size_limit: None,
            particle_size_limit: None,
            call_result_size_limit: None,
            hard_limit_enabled: false,
        }
    }
}
