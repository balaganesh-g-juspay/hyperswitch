pub(crate) mod api_error_response;
pub(crate) mod error_handlers;
pub(crate) mod utils;

use std::fmt::Display;

use actix_web::{body::BoxBody, http::StatusCode, HttpResponse, ResponseError};
use config::ConfigError;
use error_stack;
use router_env::opentelemetry::metrics::MetricsError;

pub use self::api_error_response::ApiErrorResponse;
pub(crate) use self::utils::{ApiClientErrorExt, ConnectorErrorExt, StorageErrorExt};
use crate::services;

pub type CustomResult<T, E> = error_stack::Result<T, E>;
pub type RouterResult<T> = CustomResult<T, ApiErrorResponse>;
pub type RouterResponse<T> = CustomResult<services::BachResponse<T>, ApiErrorResponse>;

// FIXME: Phase out BachResult and BachResponse
pub type BachResult<T> = Result<T, BachError>;
pub type BachResponse<T> = BachResult<services::BachResponse<T>>;

macro_rules! impl_error_display {
    ($st: ident, $arg: tt) => {
        impl std::fmt::Display for $st {
            fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                fmt.write_str(&format!(
                    "{{ error_type: {:?}, error_description: {} }}",
                    self, $arg
                ))
            }
        }
    };
}

macro_rules! impl_error_type {
    ($name: ident, $arg: tt) => {
        #[derive(Debug)]
        pub struct $name;

        impl_error_display!($name, $arg);

        impl std::error::Error for $name {}
    };
}

// FIXME: Make this a derive macro instead
macro_rules! router_error_error_stack_specific {
    ($($path: ident)::+ < $st: ident >, $($path2:ident)::* ($($inner_path2:ident)::+ <$st2:ident>) ) => {
        impl From<$($path)::+ <$st>> for BachError {
            fn from(err: $($path)::+ <$st> ) -> Self {
                $($path2)::*(err)
            }
        }
    };

    ($($path: ident)::+  <$($inner_path:ident)::+>, $($path2:ident)::* ($($inner_path2:ident)::+ <$st2:ident>) ) => {
        impl<'a> From< $($path)::+ <$($inner_path)::+> > for BachError {
            fn from(err: $($path)::+ <$($inner_path)::+> ) -> Self {
                $($path2)::*(err)
            }
        }
    };
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("DataBaseError: {0}")]
    DatabaseError(#[from] DatabaseError),

    #[error("ValueNotFound: {0}")]
    ValueNotFound(String),
    #[error("DuplicateValue: {0}")]
    DuplicateValue(String),
    #[error("KV error")]
    KVError,
}

#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("An error occurred when obtaining database connection")]
    DatabaseConnectionError,
    #[error("The requested resource was not found in the database")]
    NotFound,
    #[error("A unique constraint violation occurred")]
    UniqueViolation,
    // InsertFailed,
    #[error("An unknown error occurred")]
    Others,
}

impl_error_type!(AuthenticationError, "Authentication error");
impl_error_type!(AuthorisationError, "Authorisation error");
impl_error_type!(EncryptionError, "Encryption error");
impl_error_type!(ParsingError, "Parsing error");
impl_error_type!(UnexpectedError, "Unexpected error");
impl_error_type!(ValidateError, "validation failed");

#[derive(Debug, thiserror::Error)]
pub enum BachError {
    // Display's impl can be overridden by the attribute error marco.
    // Don't use Debug here, Debug gives error stack in response.
    #[error("{{ error_description: Error while Authenticating, error_message: {0} }}")]
    EAuthenticationError(error_stack::Report<AuthenticationError>),

    #[error("{{ error_description: Error while Authorizing, error_message: {0} }}")]
    EAuthorisationError(error_stack::Report<AuthorisationError>),

    #[error("{{ error_description: Connector implementation missing, error_message: {0} }}")]
    NotImplementedByConnector(String), //Feature not implemented by chosen connector.

    #[error("{{ error_description: Unexpected error, error_message: {0} }}")]
    EUnexpectedError(error_stack::Report<UnexpectedError>),

    #[error("{{ error_description: Error while parsing, error_message: {0} }}")]
    EParsingError(error_stack::Report<ParsingError>),

    #[error("Environment configuration error: {0}")]
    ConfigurationError(ConfigError),

    #[error("{{ error_description: Error while validating, error_message: {0} }}")]
    EValidationError(error_stack::Report<ValidateError>), // Parsing error actually

    #[error("{{ error_description: Database operation failed, error_message: {0} }}")]
    EDatabaseError(error_stack::Report<DatabaseError>),

    #[error("{{ error_description: Encryption module operation failed, error_message: {0} }}")]
    EEncryptionError(error_stack::Report<EncryptionError>),

    #[error("Metrics error: {0}")]
    EMetrics(MetricsError),

    #[error("I/O: {0}")]
    EIo(std::io::Error),
}

router_error_error_stack_specific!(
    error_stack::Report<ValidateError>,
    BachError::EValidationError(error_stack::Report<ValidateError>)
);
router_error_error_stack_specific!(
    error_stack::Report<DatabaseError>,
    BachError::EDatabaseError(error_stack::Report<DatabaseError>)
);
router_error_error_stack_specific!(
    error_stack::Report<AuthenticationError>,
    BachError::EAuthenticationError(error_stack::Report<AuthenticationError>)
);
router_error_error_stack_specific!(
    error_stack::Report<UnexpectedError>,
    BachError::EUnexpectedError(error_stack::Report<UnexpectedError>)
);
router_error_error_stack_specific!(
    error_stack::Report<ParsingError>,
    BachError::EParsingError(error_stack::Report<ParsingError>)
);
router_error_error_stack_specific!(
    error_stack::Report<EncryptionError>,
    BachError::EEncryptionError(error_stack::Report<EncryptionError>)
);

impl From<MetricsError> for BachError {
    fn from(err: MetricsError) -> Self {
        Self::EMetrics(err)
    }
}

impl From<std::io::Error> for BachError {
    fn from(err: std::io::Error) -> Self {
        Self::EIo(err)
    }
}

impl From<ring::error::Unspecified> for EncryptionError {
    fn from(_: ring::error::Unspecified) -> Self {
        Self
    }
}

impl From<ConfigError> for BachError {
    fn from(err: ConfigError) -> Self {
        Self::ConfigurationError(err)
    }
}

fn error_response<T: Display>(err: &T) -> actix_web::HttpResponse {
    actix_web::HttpResponse::BadRequest()
        .append_header(("Via", "Juspay_Router"))
        .content_type("application/json")
        .body(format!(
            "{{\n\"error\": {{\n\"message\": \"{err}\" \n}} \n}}\n"
        ))
}

impl ResponseError for BachError {
    fn status_code(&self) -> StatusCode {
        match self {
            BachError::EParsingError(_)
            | BachError::EAuthenticationError(_)
            | BachError::EAuthorisationError(_)
            | BachError::EValidationError(_) => StatusCode::BAD_REQUEST,

            BachError::EDatabaseError(_)
            | BachError::NotImplementedByConnector(_)
            | BachError::EMetrics(_)
            | BachError::EIo(_)
            | BachError::ConfigurationError(_)
            | BachError::EEncryptionError(_)
            | BachError::EUnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> actix_web::HttpResponse {
        error_response(self)
    }
}

pub fn http_not_implemented() -> HttpResponse<BoxBody> {
    ApiErrorResponse::NotImplemented.error_response()
}

#[derive(Debug, thiserror::Error)]
pub enum ApiClientError {
    #[error("Header map construction failed")]
    HeaderMapConstructionFailed,
    #[error("Invalid proxy configuration")]
    InvalidProxyConfiguration,
    #[error("Client construction failed")]
    ClientConstructionFailed,

    #[error("URL encoding of request payload failed")]
    UrlEncodingFailed,
    #[error("Failed to send request to connector")]
    RequestNotSent,
    #[error("Failed to decode response")]
    ResponseDecodingFailed,

    #[error("Server responded with Bad Request")]
    BadRequestReceived(bytes::Bytes),
    #[error("Server responded with Unauthorized")]
    UnauthorizedReceived(bytes::Bytes),
    #[error("Server responded with Forbidden")]
    ForbiddenReceived,
    #[error("Server responded with Not Found")]
    NotFoundReceived(bytes::Bytes),
    #[error("Server responded with Method Not Allowed")]
    MethodNotAllowedReceived,
    #[error("Server responded with Request Timeout")]
    RequestTimeoutReceived,
    #[error("Server responded with Unprocessable Entity")]
    UnprocessableEntityReceived(bytes::Bytes),
    #[error("Server responded with Too Many Requests")]
    TooManyRequestsReceived,

    #[error("Server responded with Internal Server Error")]
    InternalServerErrorReceived,
    #[error("Server responded with Bad Gateway")]
    BadGatewayReceived,
    #[error("Server responded with Service Unavailable")]
    ServiceUnavailableReceived,
    #[error("Server responded with Gateway Timeout")]
    GatewayTimeoutReceived,
    #[error("Server responded with unexpected response")]
    UnexpectedServerResponse,
}

#[derive(Debug, thiserror::Error)]
pub enum ConnectorError {
    #[error("Error while obtaining URL for the integration")]
    FailedToObtainIntegrationUrl,
    #[error("Failed to encode connector request")]
    RequestEncodingFailed,
    #[error("Failed to deserialize connector response")]
    ResponseDeserializationFailed,
    #[error("Failed to execute a processing step: {0:?}")]
    ProcessingStepFailed(Option<bytes::Bytes>),
    #[error("The connector returned an unexpected response: {0:?}")]
    UnexpectedResponseError(bytes::Bytes),
    #[error("Failed to parse custom routing rules from merchant account")]
    RoutingRulesParsingError,
    #[error("Failed to obtain preferred connector from merchant account")]
    FailedToObtainPreferredConnector,
    #[error("An invalid connector name was provided")]
    InvalidConnectorName,
    #[error("Failed to handle connector response")]
    ResponseHandlingFailed,
    #[error("Missing required field: {field_name}")]
    MissingRequiredField { field_name: String },
    #[error("Failed to obtain authentication type")]
    FailedToObtainAuthType,
    #[error("This step has not been implemented for: {0}")]
    NotImplemented(String),
    #[error("Webhooks not implemented for this connector")]
    WebhooksNotImplemented,
    #[error("Failed to decode webhook event body")]
    WebhookBodyDecodingFailed,
    #[error("Signature not found for incoming webhook")]
    WebhookSignatureNotFound,
    #[error("Failed to verify webhook source")]
    WebhookSourceVerificationFailed,
    #[error("Could not find merchant secret in DB for incoming webhook source verification")]
    WebhookVerificationSecretNotFound,
    #[error("Incoming webhook object reference ID not found")]
    WebhookReferenceIdNotFound,
    #[error("Incoming webhook event type not found")]
    WebhookEventTypeNotFound,
    #[error("Incoming webhook event resource object not found")]
    WebhookResourceObjectNotFound,
}

#[derive(Debug, thiserror::Error)]
pub enum CardVaultError {
    #[error("Failed to save card in card vault")]
    SaveCardFailed,
    #[error("Failed to fetch card details from card vault")]
    FetchCardFailed,
    #[error("Failed to encode card vault request")]
    RequestEncodingFailed,
    #[error("Failed to deserialize card vault response")]
    ResponseDeserializationFailed,
    #[error("Failed to create payment method")]
    PaymentMethodCreationFailed,
    #[error("Missing required field: {field_name}")]
    MissingRequiredField { field_name: String },
    #[error("The card vault returned an unexpected response: {0:?}")]
    UnexpectedResponseError(bytes::Bytes),
}

#[derive(Debug, thiserror::Error)]
pub enum ProcessTrackerError {
    #[error("An unexpected flow was specified")]
    UnexpectedFlow,
    #[error("Failed to serialize object")]
    SerializationFailed,
    #[error("Failed to deserialize object")]
    DeserializationFailed,
    #[error("Missing required field")]
    MissingRequiredField,
    #[error("Failed to insert process batch into stream")]
    BatchInsertionFailed,
    #[error("Failed to insert process into stream")]
    ProcessInsertionFailed,
    #[error("The process batch with the specified details was not found")]
    BatchNotFound,
    #[error("Failed to update process batch in stream")]
    BatchUpdateFailed,
    #[error("Failed to delete process batch from stream")]
    BatchDeleteFailed,
    #[error("An error occurred when trying to read process tracker configuration")]
    ConfigurationError,
    #[error("Failed to update process in database")]
    ProcessUpdateFailed,
    #[error("Failed to fetch processes from database")]
    ProcessFetchingFailed,
    #[error("Failed while fetching: {resource_name}")]
    ResourceFetchingFailed { resource_name: String },
    #[error("Failed while executing: {flow}")]
    FlowExecutionError { flow: String },
    #[error("Not Implemented")]
    NotImplemented,
    #[error("Recieved Error ApiResponseError: {0}")]
    EApiErrorResponse(error_stack::Report<ApiErrorResponse>),
    #[error("Recieved Error StorageError: {0}")]
    EStorageError(error_stack::Report<StorageError>),
    #[error("Recieved Error RedisError: {0}")]
    ERedisError(error_stack::Report<RedisError>),
    #[error("Recieved Error ParsingError: {0}")]
    EParsingError(error_stack::Report<ParsingError>),
    #[error("Validation Error Recieved: {0}")]
    EValidationError(error_stack::Report<ValidationError>),
}

macro_rules! error_to_process_tracker_error {
    ($($path: ident)::+ < $st: ident >, $($path2:ident)::* ($($inner_path2:ident)::+ <$st2:ident>) ) => {
        impl From<$($path)::+ <$st>> for ProcessTrackerError {
            fn from(err: $($path)::+ <$st> ) -> Self {
                $($path2)::*(err)
            }
        }
    };

    ($($path: ident)::+  <$($inner_path:ident)::+>, $($path2:ident)::* ($($inner_path2:ident)::+ <$st2:ident>) ) => {
        impl<'a> From< $($path)::+ <$($inner_path)::+> > for ProcessTrackerError {
            fn from(err: $($path)::+ <$($inner_path)::+> ) -> Self {
                $($path2)::*(err)
            }
        }
    };
}

error_to_process_tracker_error!(
    error_stack::Report<ApiErrorResponse>,
    ProcessTrackerError::EApiErrorResponse(error_stack::Report<ApiErrorResponse>)
);

error_to_process_tracker_error!(
    error_stack::Report<StorageError>,
    ProcessTrackerError::EStorageError(error_stack::Report<StorageError>)
);

error_to_process_tracker_error!(
    error_stack::Report<RedisError>,
    ProcessTrackerError::ERedisError(error_stack::Report<RedisError>)
);

error_to_process_tracker_error!(
    error_stack::Report<ParsingError>,
    ProcessTrackerError::EParsingError(error_stack::Report<ParsingError>)
);

error_to_process_tracker_error!(
    error_stack::Report<ValidationError>,
    ProcessTrackerError::EValidationError(error_stack::Report<ValidationError>)
);

#[derive(Debug, thiserror::Error)]
pub enum RedisError {
    #[error("Failed to set key value in Redis")]
    SetFailed,
    #[error("Failed to set key value with expiry in Redis")]
    SetExFailed,
    #[error("Failed to set expiry for key value in Redis")]
    SetExpiryFailed,
    #[error("Failed to get key value in Redis")]
    GetFailed,
    #[error("Failed to delete key value in Redis")]
    DeleteFailed,
    #[error("Failed to append entry to redis stream")]
    StreamAppendFailed,
    #[error("Failed to read entries from redis stream")]
    StreamReadFailed,
    #[error("Failed to delete entries from redis stream")]
    StreamDeleteFailed,
    #[error("Failed to acknowledge redis stream entry")]
    StreamAcknowledgeFailed,
    #[error("Failed to create redis consumer group")]
    ConsumerGroupCreateFailed,
    #[error("Failed to destroy redis consumer group")]
    ConsumerGroupDestroyFailed,
    #[error("Failed to delete consumer from consumer group")]
    ConsumerGroupRemoveConsumerFailed,
    #[error("Failed to set last ID on consumer group")]
    ConsumerGroupSetIdFailed,
    #[error("Failed to set redis stream message owner")]
    ConsumerGroupClaimFailed,
    #[error("Failed to serialize application type to json")]
    JsonSerializationFailed,
    #[error("Failed to deserialize application type from json")]
    JsonDeserializationFailed,
}

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Missing required field: {field_name}")]
    MissingRequiredField { field_name: String },
    #[error("Incorrect value provided for field: {field_name}")]
    IncorrectValueProvided { field_name: &'static str },
}

#[derive(Debug, thiserror::Error)]
pub enum WebhooksFlowError {
    #[error("Merchant webhook config not found")]
    MerchantConfigNotFound,
    #[error("Webhook details for merchant not configured")]
    MerchantWebhookDetailsNotFound,
    #[error("Merchant does not have a webhook URL configured")]
    MerchantWebhookURLNotConfigured,
    #[error("Payments core flow failed")]
    PaymentsCoreFailed,
    #[error("Webhook event creation failed")]
    WebhookEventCreationFailed,
    #[error("Unable to fork webhooks flow for outgoing webhooks")]
    ForkFlowFailed,
    #[error("Webhook api call to merchant failed")]
    CallToMerchantFailed,
    #[error("Webhook not received by merchant")]
    NotReceivedByMerchant,
}

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("Failed to encode given message")]
    EncodingFailed,
    #[error("Failed to decode given message")]
    DecodingFailed,
    #[error("Failed to sign message")]
    MessageSigningFailed,
    #[error("Failed to verify signature")]
    SignatureVerificationFailed,
}