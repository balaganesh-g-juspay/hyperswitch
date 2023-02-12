use serde::{Deserialize, Serialize};
use crate::{core::errors,types::{self,api, storage::enums}};
use masking::{Secret};

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct OpayoCard{
   card : OpayoCardSession,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OpayoCardSession{
   merchant_session_key : String,
   card_identifier : String,
   reusable : bool,
   save : bool,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BillingAddress{
   address1 : Secret<String>,
   city : String,
   country : String,
   postalCode : Secret<String>
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct CustomerAuthentication{
   notificationURL : String,
   browserIP : String,
   browserAcceptHeader : String,
   browserJavascriptEnabled : bool,
   browserLanguage : String,
   browserUserAgent : String,
   challengeWindowSize : String,
   transType : String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct CredentialType{
   coUsage : String,
   initiatedType : String,
   mitType : String,
   recurringExpiry : String,
   recurringFrequency : String,
   purchaseInstalData : String,
}
//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OpayoPaymentsRequest {
   transaction_type : String,
   payment_method : OpayoCard,
   vendor_tx_code : String,
   amount : i64,
   currency : String,
   description : String,
   customer_first_name : Secret<String>,
   customer_last_name : Secret<String>,
   billing_address : BillingAddress,
   apply3_d_secure : String,
   strong_customer_authentication : CustomerAuthentication,
   credential_type : Option<CredentialType>,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for OpayoPaymentsRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_item: &types::PaymentsAuthorizeRouterData) -> Result<Self,Self::Error> {
        let amount = _item.request.amount;
        let currency = format!("{:?}", _item.request.currency);
        let description = _item.description.clone().ok_or(errors::ConnectorError::MissingRequiredField{field_name: "item.description",},)?;
        let vendor_tx_code = _item.payment_id.clone();
        let payment_method = match _item.request.payment_method_data.clone() {
            api::PaymentMethod::Card(_) => Ok(OpayoCard{
                card: OpayoCardSession{
                    merchant_session_key : String::from("No idea"),
                    card_identifier : String::from("No idea"),
                    reusable : false,
                    save : false,
                }
            }),
            _ => Err(errors::ConnectorError::NotImplemented(
                "Unknown payment method".to_string(),
            )),
        }?;
        let browser_info = _item.request.browser_info.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "browser_info",
            },
        )?;
        let notification_url = _item.return_url.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "notification_url",
            },
        )?;

        let address = _item.address.billing.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "billing_address",
            },
        )?;

        let actualAddress = address.address.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "billing_address",
            },
        )?;

        let billing_address = BillingAddress{
            address1 : actualAddress.line1.clone().ok_or(errors::ConnectorError::MissingRequiredField {field_name: "address1",})?,
            city : actualAddress.city.clone().ok_or(errors::ConnectorError::MissingRequiredField {field_name: "city",})?,
            country : actualAddress.country.clone().ok_or(errors::ConnectorError::MissingRequiredField {field_name: "country",})?,
            postalCode : actualAddress.zip.clone().ok_or(errors::ConnectorError::MissingRequiredField {field_name: "zip",})?,
         };
        let ipAddr = browser_info.ip_address.clone().ok_or(errors::ConnectorError::MissingRequiredField {field_name: "browserIP",})?;
        let strong_customer_authentication = CustomerAuthentication{
            notificationURL : notification_url,
            browserIP : ipAddr.to_string(),
            browserAcceptHeader : browser_info.accept_header,
            browserJavascriptEnabled : browser_info.java_script_enabled,
            browserLanguage : browser_info.language,
            browserUserAgent : browser_info.user_agent,
            challengeWindowSize : String::from(getWindowSize(browser_info.screen_width)),
            transType : String::from("GoodsAndServicePurchase"),
         };

        Ok(Self{
            transaction_type : String::from("Payment"),
            payment_method : payment_method,
            vendor_tx_code : vendor_tx_code,
            amount : amount,
            currency : currency,
            description : description,
            customer_first_name : actualAddress.first_name.clone().ok_or(errors::ConnectorError::MissingRequiredField {field_name: "first_name",})?,
            customer_last_name : actualAddress.last_name.clone().ok_or(errors::ConnectorError::MissingRequiredField {field_name: "last_name",})?,
            billing_address : billing_address,
            apply3_d_secure : String::from("UseMSPSetting"),
            strong_customer_authentication : strong_customer_authentication,
            credential_type : None,
        })

        }
    }


fn fromOption(o : Option<String>, default : String) -> String{
    match o{
        Some (val) => val,
        None => default,
    }
 }

fn getWindowSize(width : u32) -> &'static str {
    if width <= 250 {return "Small"}
    else if width <= 390{return "Medium"}
    else if width <= 500{return "Large"}
    else if width <= 600{return "ExtraLarge"}
    else {return "FullScreen"};
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct OpayoAuthType {
    pub(super) api_key: String
}

impl TryFrom<&types::ConnectorAuthType> for OpayoAuthType  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::HeaderKey { api_key } = _auth_type {
            Ok(Self {
                api_key: api_key.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OpayoPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<OpayoPaymentStatus> for enums::AttemptStatus {
    fn from(item: OpayoPaymentStatus) -> Self {
        match item {
            OpayoPaymentStatus::Succeeded => Self::Charged,
            OpayoPaymentStatus::Failed => Self::Failure,
            OpayoPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OpayoPaymentsResponse {
    status: OpayoPaymentStatus,
    id: String,
}

impl<F,T> TryFrom<types::ResponseRouterData<F, OpayoPaymentsResponse, T, types::PaymentsResponseData>> for types::RouterData<F, T, types::PaymentsResponseData> {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(item: types::ResponseRouterData<F, OpayoPaymentsResponse, T, types::PaymentsResponseData>) -> Result<Self,Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: None,
                redirect: false,
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct OpayoRefundRequest {}

impl<F> TryFrom<&types::RefundsRouterData<F>> for OpayoRefundRequest {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(_item: &types::RefundsRouterData<F>) -> Result<Self,Self::Error> {
       todo!()
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Succeeded => Self::Success,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Processing => Self::Pending,
            //TODO: Review mapping
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        _item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>> for types::RefundsRouterData<api::RSync>
{
     type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(_item: types::RefundsResponseRouterData<api::RSync, RefundResponse>) -> Result<Self,Self::Error> {
         todo!()
     }
 }

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct OpayoErrorResponse {}
