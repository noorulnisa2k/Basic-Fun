use serde::{Deserialize, Serialize};
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

    #[serde(rename = "NumAtCard")]
    pub num_at_card: Option<String>,

    #[serde(rename = "Series")]
    pub series: Option<i32>,

    #[serde(rename = "TaxDate")]
    pub tax_date: Option<String>,

    #[serde(rename = "ShipToCode")]
    pub ship_to_code: Option<String>,

    #[serde(rename = "PayToCode")]
    pub pay_to_code: Option<String>,

    #[serde(rename = "BPL_IDAssignedToInvoice")]
    pub bpl_assigned_to_invoice: Option<i32>,

    #[serde(rename = "U_Warehouse_Order")]
    pub u_warehouse_order: Option<String>,

    #[serde(rename = "U_Warehouse_Order_Date")]
    pub u_warehouse_order_date: Option<String>,

    #[serde(rename = "U_Warehouse_Order_Process")]
    pub u_warehouse_order_process: Option<String>,

    #[serde(rename = "Document_ApprovalRequests")]
    pub document_approval_requests: Option<Vec<Value>>,

    #[serde(rename = "DocumentLines")]
    pub document_lines: Vec<DocumentLine>, 

    #[serde(rename = "AddressExtension")]
    pub address_extension: Option<AddressExtension>,

    #[serde(rename = "U_BillingType")]
    pub u_billing_type: Option<String>,

    #[serde(rename = "TransportationCode")]
    pub u_transportation_code: Option<i64>,
    
    #[serde(rename = "TrnspCode")]
    pub trnsp_code: Option<i64>,

    #[serde(rename = "DocCurrency")]
    pub doc_currency: Option<String>,

    #[serde(rename = "U_U_WSO")]
    pub u_u_wso: Option<String>,

    #[serde(rename = "ShipFrom")]
    pub ship_from: Option<String>,

    #[serde(rename = "TotalDiscount")]
    pub total_discount: Option<f64>,

     #[serde(rename = "DiscountPercentage")]
    pub discount_percentage: Option<i64>,

    #[serde(rename = "ExtraDays")]
    pub extra_days: Option<i64>,

    #[serde(rename = "U_U_Department")]
    pub u_u_department: Option<i64>,
}


#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AddressExtension {
    #[serde(rename = "ShipToStreet")]
    pub ship_to_street: Option<String>,

    #[serde(rename = "ShipToStreetNo")]
    pub ship_to_street_no: Option<String>,

    #[serde(rename = "ShipToBlock")]
    pub ship_to_block: Option<String>,

    #[serde(rename = "ShipToBuilding")]
    pub ship_to_building: Option<String>,

    #[serde(rename = "ShipToCity")]
    pub ship_to_city: Option<String>,

    #[serde(rename = "ShipToZipCode")]
    pub ship_to_zip_code: Option<String>,

    #[serde(rename = "ShipToCounty")]
    pub ship_to_county: Option<String>,

    #[serde(rename = "ShipToState")]
    pub ship_to_state: Option<String>,

    #[serde(rename = "ShipToCountry")]
    pub ship_to_country: Option<String>,

    #[serde(rename = "BillToStreet")]
    pub bill_to_street: Option<String>,

    #[serde(rename = "BillToBlock")]
    pub bill_to_block: Option<String>,

    #[serde(rename = "BillToCity")]
    pub bill_to_city: Option<String>,

    #[serde(rename = "BillToZipCode")]
    pub bill_to_zip_code: Option<String>,

    #[serde(rename = "BillToState")]
    pub bill_to_state: Option<String>,

    #[serde(rename = "CountryB")]
    pub country_b: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentLine {
    #[serde(rename = "LineNum")]
    pub line_num: Option<usize>,

    #[serde(rename = "ShipDate")]
    pub ship_date: Option<String>,

    #[serde(rename = "ItemCode")]
    pub item_code: Option<String>,

    #[serde(rename = "ItemDescription")]
    pub item_description: Option<String>,

    #[serde(rename = "Quantity")]
    pub quantity: Option<f64>,

    #[serde(rename = "WarehouseCode")]
    pub warehouse_code: Option<String>,

    #[serde(rename = "UnitPrice")]
    pub unit_price: Option<i64>,

    #[serde(rename = "Price")]
    pub price: Option<f64>,

    #[serde(rename = "TaxCode")]
    pub tax_code: Option<String>,

    #[serde(rename = "U_ACW_DeliveryFrom")]
    pub u_acw_delivery_from: Option<String>,

    #[serde(rename = "U_ACW_DeliveryEnd")]
    pub u_acw_delivery_end: Option<String>,

    #[serde(rename = "U_TBD_Cust_No")]
    pub u_tbd_cust_no: Option<String>,

    #[serde(rename = "SupplierCatNum")]
    pub supplier_cat_num: Option<String>,

    #[serde(rename = "U_TBD_Cust_Dept_No")]
    pub u_tbd_cust_dept_no: Option<i64>,

    #[serde(rename = "U_TBD_SO_Ref")]
    pub u_tbd_so_ref: Option<String>,
}