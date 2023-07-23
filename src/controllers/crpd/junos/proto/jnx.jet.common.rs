/// \[brief\]: Message representing timeval structure
/// \[detail\]: Message representing timeval structure
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TimeVal {
    /// \[brief\]: Seconds from timeval structure
    #[prost(uint64, tag = "1")]
    pub seconds: u64,
    /// \[brief\]: Microseconds from timeval structure
    #[prost(uint64, tag = "2")]
    pub microseconds: u64,
}
/// \[brief\]: RPC execution status information
/// \[detail\]: RPC execution status information
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RpcStatus {
    /// \[brief\]: Numerical code indicating success or failure of an RPC
    #[prost(enumeration = "StatusCode", tag = "1")]
    pub code: i32,
    /// \[brief\]: Informational message string to convey reason for RPC failure
    #[prost(string, tag = "2")]
    pub message: ::prost::alloc::string::String,
}
/// \[brief\]: Numeric ranges can be used to provide range of unsigned 32-bit values.
/// \[detail\]: Numeric ranges can be used to provide range of unsigned 32-bit values.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct NumericRange {
    /// \[brief\]: Range Minimum value (inclusive).
    /// \[mandatory\]:
    #[prost(uint32, tag = "1")]
    pub min: u32,
    /// \[brief\]: Range Maximum value (inclusive).
    /// \[mandatory\]:
    #[prost(uint32, tag = "2")]
    pub max: u32,
}
/// \[brief\]: List of Numeric Range.
/// \[detail\]: List of Numeric Range.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct NumericRangeList {
    /// \[brief\]: Range List for enums.
    /// OPTIONAL
    #[prost(message, repeated, tag = "1")]
    pub range_list: ::prost::alloc::vec::Vec<NumericRange>,
}
/// \[brief\]: Global status codes to be returned in response messages.
/// \[detail\]: Global status codes to be returned in response messages.
/// Per-RPC specific status/error codes are to be conveyed
/// in sub-codes defined in respective API definitions.
/// \[default\]: SUCCESS
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum StatusCode {
    /// \[brief\]: Indicates that the RPC executed without error
    Success = 0,
    /// \[brief\]: Indicates a failure condition that should be treated as fatal
    Failure = 1,
}
impl StatusCode {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            StatusCode::Success => "SUCCESS",
            StatusCode::Failure => "FAILURE",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "SUCCESS" => Some(Self::Success),
            "FAILURE" => Some(Self::Failure),
            _ => None,
        }
    }
}
