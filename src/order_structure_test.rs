use serde::{de, Deserialize, Deserializer, Serialize};
use serde_json::Value;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Orders {
    // #[serde(rename = "DocEntry")]
    // pub doc_entry: Option<usize>,   // ✅

    #[serde(rename = "DocDate")]
    pub doc_date: String,

    #[serde(rename = "DocDueDate")]
    pub doc_due_date: String,

    #[serde(rename = "CardCode")]
    pub card_code: String,

    // #[serde(rename = "CardName")]
    // pub card_name: Option<String>,

    #[serde(rename = "NumAtCard", skip_serializing_if = "Option::is_none")]
    pub num_at_card: Option<String>,

    #[serde(rename = "Series", skip_serializing_if = "Option::is_none")]
    pub series: Option<i32>,

    #[serde(rename = "TaxDate", skip_serializing_if = "Option::is_none")]
    pub tax_date: Option<String>,

    #[serde(rename = "ShipToCode", skip_serializing_if = "Option::is_none",)]
    pub ship_to_code: Option<String>,

    #[serde(rename = "PayToCode", skip_serializing_if = "Option::is_none")]
    pub pay_to_code: Option<String>,

    #[serde(rename = "BPL_IDAssignedToInvoice", skip_serializing_if = "Option::is_none")]
    pub bpl_assigned_to_invoice: Option<i32>,

    #[serde(rename = "U_Warehouse_Order", skip_serializing_if = "Option::is_none")]
    pub u_warehouse_order: Option<String>,

    #[serde(rename = "U_Warehouse_Order_Date", skip_serializing_if = "Option::is_none")]
    pub u_warehouse_order_date: Option<String>,

    #[serde(rename = "U_Warehouse_Order_Process", skip_serializing_if = "Option::is_none")]
    pub u_warehouse_order_process: Option<String>,

    #[serde(rename = "Document_ApprovalRequests", skip_serializing_if = "Option::is_none")]
    pub document_approval_requests: Option<Vec<Value>>,

    #[serde(rename = "DocumentLines")]
    pub document_lines: Vec<DocumentLine>, 

    #[serde(rename = "AddressExtension", skip_serializing_if = "Option::is_none")]
    pub address_extension: Option<AddressExtension>,

    #[serde(rename = "U_BillingType", skip_serializing_if = "Option::is_none")]
    pub u_billing_type: Option<String>,

    #[serde(rename = "TransportationCode", skip_serializing_if = "Option::is_none")]
    pub u_transportation_code: Option<String>,

    #[serde(rename = "U_SHIP_SCAC", skip_serializing_if = "Option::is_none")]
    pub u_ship_scac: Option<String>,
    
    #[serde(rename = "TrnspCode", skip_serializing_if = "Option::is_none")]
    pub trnsp_code: Option<i64>,

    #[serde(rename = "DocCurrency", skip_serializing_if = "Option::is_none")]
    pub doc_currency: Option<String>,

    #[serde(rename = "U_U_WSO", skip_serializing_if = "Option::is_none")]
    pub u_u_wso: Option<String>,

    #[serde(rename = "ShipFrom", skip_serializing_if = "Option::is_none")]
    pub ship_from: Option<String>,

    #[serde(rename = "TotalDiscount", skip_serializing_if = "Option::is_none")]
    pub total_discount: Option<f64>,

    #[serde(rename = "DiscountPercentage", skip_serializing_if = "Option::is_none")]
    pub discount_percentage: Option<i64>,

    #[serde(rename = "ExtraDays", skip_serializing_if = "Option::is_none")]
    pub extra_days: Option<i64>,

    #[serde(rename = "U_U_Department", skip_serializing_if = "Option::is_none")]
    pub u_u_department: Option<i64>,
}


#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AddressExtension {
    #[serde(rename = "ShipToStreet", skip_serializing_if = "Option::is_none")]
    pub ship_to_street: Option<String>,

    #[serde(rename = "ShipToStreetNo", skip_serializing_if = "Option::is_none")]
    pub ship_to_street_no: Option<String>,

    #[serde(rename = "ShipToBlock", skip_serializing_if = "Option::is_none")]
    pub ship_to_block: Option<String>,

    #[serde(rename = "ShipToBuilding", skip_serializing_if = "Option::is_none")]
    pub ship_to_building: Option<String>,

    #[serde(rename = "ShipToCity", skip_serializing_if = "Option::is_none")]
    pub ship_to_city: Option<String>,

    #[serde(rename = "ShipToZipCode", skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_option_string_from_number")]
    pub ship_to_zip_code: Option<String>,

    #[serde(rename = "ShipToCounty", skip_serializing_if = "Option::is_none")]
    pub ship_to_county: Option<String>,

    #[serde(rename = "ShipToState", skip_serializing_if = "Option::is_none")]
    pub ship_to_state: Option<String>,

    #[serde(rename = "ShipToCountry", skip_serializing_if = "Option::is_none")]
    pub ship_to_country: Option<String>,

    #[serde(rename = "BillToStreet", skip_serializing_if = "Option::is_none")]
    pub bill_to_street: Option<String>,

    #[serde(rename = "BillToBlock", skip_serializing_if = "Option::is_none")]
    pub bill_to_block: Option<String>,

    #[serde(rename = "BillToCity", skip_serializing_if = "Option::is_none")]
    pub bill_to_city: Option<String>,

    #[serde(rename = "BillToZipCode", skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_option_string_from_number")]
    pub bill_to_zip_code: Option<String>,

    #[serde(rename = "BillToState", skip_serializing_if = "Option::is_none")]
    pub bill_to_state: Option<String>,

    #[serde(rename = "CountryB", skip_serializing_if = "Option::is_none")]
    pub country_b: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentLine {
    #[serde(rename = "LineNum", skip_serializing_if = "Option::is_none")]
    pub line_num: Option<usize>,

    #[serde(rename = "ShipDate", skip_serializing_if = "Option::is_none")]
    pub ship_date: Option<String>,

    #[serde(rename = "ItemCode", skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_option_string_from_number")]
    pub item_code: Option<String>,

    #[serde(rename = "ItemDescription", skip_serializing_if = "Option::is_none")]
    pub item_description: Option<String>,

    #[serde(rename = "Quantity", skip_serializing_if = "Option::is_none")]
    pub quantity: Option<f64>,

    #[serde(rename = "WarehouseCode", skip_serializing_if = "Option::is_none")]
    pub warehouse_code: Option<String>,

    #[serde(rename = "UnitPrice", skip_serializing_if = "Option::is_none")]
    pub unit_price: Option<f64>,

    #[serde(rename = "Price", skip_serializing_if = "Option::is_none")]
    pub price: Option<f64>,

    #[serde(rename = "TaxCode", skip_serializing_if = "Option::is_none")]
    pub tax_code: Option<String>,

    #[serde(rename = "U_ACW_DeliveryFrom", skip_serializing_if = "Option::is_none")]
    pub u_acw_delivery_from: Option<String>,

    #[serde(rename = "U_ACW_DeliveryEnd", skip_serializing_if = "Option::is_none")]
    pub u_acw_delivery_end: Option<String>,

    #[serde(rename = "U_TBD_Cust_No", skip_serializing_if = "Option::is_none")]
    pub u_tbd_cust_no: Option<String>,

    #[serde(rename = "SupplierCatNum", skip_serializing_if = "Option::is_none")]
    pub supplier_cat_num: Option<String>,

    #[serde(rename = "LineTotal", skip_serializing_if = "Option::is_none")]
    pub line_total: Option<f64>,

    #[serde(rename = "U_TBD_Cust_Dept_No", skip_serializing_if = "Option::is_none")]
    pub u_tbd_cust_dept_no: Option<i64>,

    #[serde(rename = "U_TBD_SO_Ref", skip_serializing_if = "Option::is_none")]
    pub u_tbd_so_ref: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Token {
    #[serde(rename = "odata.metadata")]
    pub odata_metadata: Option<String>,
    #[serde(rename = "SessionId")]
    pub session_id: String,
    #[serde(rename = "Version")]
    pub version: Option<String>,
    #[serde(rename = "SessionTimeout")]
    pub session_timeout: Option<i64>,
}

fn deserialize_option_string_from_number<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Option<Value> = Option::deserialize(deserializer)?;

    match value {
        Some(Value::Number(num)) => Ok(Some(num.to_string())),
        Some(Value::String(s)) => Ok(Some(s)),
        None => Ok(None),
        _ => Err(de::Error::custom("Expected a number or string")),
    }
}
