// use serde_derive::Deserialize;
// use serde_derive::Serialize;
// use serde_json::Value;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    #[serde(rename = "odata.metadata", skip_serializing_if = "Option::is_none")]
    pub odata_metadata: Option<String>,
    #[serde(rename = "value")]
    pub orders: Value,
    // pub orders_json: Option<Vec<Orders>>,
}

// Deserialize only necessary elements
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Orders {
    #[serde(rename = "DocEntry")]
    pub doc_entry: usize,
    #[serde(rename = "CardCode")]
    pub card_code: String,
    #[serde(rename = "CardName")]
    pub card_name: String,
    #[serde(rename = "NumAtCard")]
    pub num_at_card: Option<String>,
    #[serde(rename = "BPLName")]
    pub bplname: String,
    #[serde(rename = "AddressExtension", skip_serializing_if = "Option::is_none")]
    pub address_extension: Option<AddressExtension>,
    #[serde(rename = "DocumentLines")]
    pub document_lines: Vec<DocumentLine>,
    #[serde(rename = "DocNum")]
    pub doc_num: usize,
    #[serde(rename = "DocCurrency", skip_serializing_if = "Option::is_none")]
    pub doc_currency: Option<String>,
}
/*
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, skip_serializing_if = "Option::is_none")]
#[serde(rename_all = "camelCase", skip_serializing_if = "Option::is_none")]
pub struct Orders {
    #[serde(rename = "odata.etag", skip_serializing_if = "Option::is_none")]
    pub odata_etag: Option<String>,
    #[serde(rename = "DocEntry", skip_serializing_if = "Option::is_none")]
    pub doc_entry: i64,
    #[serde(rename = "DocNum", skip_serializing_if = "Option::is_none")]
    pub doc_num: i64,
    #[serde(rename = "DocType", skip_serializing_if = "Option::is_none")]
    pub doc_type: String,
    #[serde(rename = "HandWritten", skip_serializing_if = "Option::is_none")]
    pub hand_written: String,
    #[serde(rename = "Printed", skip_serializing_if = "Option::is_none")]
    pub printed: String,
    #[serde(rename = "DocDate", skip_serializing_if = "Option::is_none")]
    pub doc_date: String,
    #[serde(rename = "DocDueDate", skip_serializing_if = "Option::is_none")]
    pub doc_due_date: String,
    #[serde(rename = "CardCode", skip_serializing_if = "Option::is_none")]
    pub card_code: String,
    #[serde(rename = "CardName", skip_serializing_if = "Option::is_none")]
    pub card_name: String,
    #[serde(rename = "Address", skip_serializing_if = "Option::is_none")]
    pub address: String,
    #[serde(rename = "NumAtCard", skip_serializing_if = "Option::is_none")]
    pub num_at_card: String,
    #[serde(rename = "DocTotal", skip_serializing_if = "Option::is_none")]
    pub doc_total: f64,
    #[serde(rename = "AttachmentEntry", skip_serializing_if = "Option::is_none")]
    pub attachment_entry: Value,
    #[serde(rename = "DocCurrency", skip_serializing_if = "Option::is_none")]
    pub doc_currency: String,
    #[serde(rename = "DocRate", skip_serializing_if = "Option::is_none")]
    pub doc_rate: f64,
    #[serde(rename = "Reference1", skip_serializing_if = "Option::is_none")]
    pub reference1: String,
    #[serde(rename = "Reference2", skip_serializing_if = "Option::is_none")]
    pub reference2: Value,
    #[serde(rename = "Comments", skip_serializing_if = "Option::is_none")]
    pub comments: String,
    #[serde(rename = "JournalMemo", skip_serializing_if = "Option::is_none")]
    pub journal_memo: String,
    #[serde(rename = "PaymentGroupCode", skip_serializing_if = "Option::is_none")]
    pub payment_group_code: i64,
    #[serde(rename = "DocTime", skip_serializing_if = "Option::is_none")]
    pub doc_time: String,
    #[serde(rename = "SalesPersonCode", skip_serializing_if = "Option::is_none")]
    pub sales_person_code: i64,
    #[serde(rename = "TransportationCode", skip_serializing_if = "Option::is_none")]
    pub transportation_code: i64,
    #[serde(rename = "Confirmed", skip_serializing_if = "Option::is_none")]
    pub confirmed: String,
    #[serde(rename = "ImportFileNum", skip_serializing_if = "Option::is_none")]
    pub import_file_num: i64,
    #[serde(rename = "SummeryType", skip_serializing_if = "Option::is_none")]
    pub summery_type: String,
    #[serde(rename = "ContactPersonCode", skip_serializing_if = "Option::is_none")]
    pub contact_person_code: i64,
    #[serde(rename = "ShowSCN", skip_serializing_if = "Option::is_none")]
    pub show_scn: String,
    #[serde(rename = "Series", skip_serializing_if = "Option::is_none")]
    pub series: i64,
    #[serde(rename = "TaxDate", skip_serializing_if = "Option::is_none")]
    pub tax_date: String,
    #[serde(rename = "PartialSupply", skip_serializing_if = "Option::is_none")]
    pub partial_supply: String,
    #[serde(rename = "DocObjectCode", skip_serializing_if = "Option::is_none")]
    pub doc_object_code: String,
    #[serde(rename = "ShipToCode", skip_serializing_if = "Option::is_none")]
    pub ship_to_code: String,
    #[serde(rename = "Indicator", skip_serializing_if = "Option::is_none")]
    pub indicator: Value,
    #[serde(rename = "FederalTaxID", skip_serializing_if = "Option::is_none")]
    pub federal_tax_id: Value,
    #[serde(rename = "DiscountPercent", skip_serializing_if = "Option::is_none")]
    pub discount_percent: f64,
    #[serde(rename = "PaymentReference", skip_serializing_if = "Option::is_none")]
    pub payment_reference: String,
    #[serde(rename = "CreationDate", skip_serializing_if = "Option::is_none")]
    pub creation_date: String,
    #[serde(rename = "UpdateDate", skip_serializing_if = "Option::is_none")]
    pub update_date: String,
    #[serde(rename = "FinancialPeriod", skip_serializing_if = "Option::is_none")]
    pub financial_period: i64,
    #[serde(rename = "UserSign", skip_serializing_if = "Option::is_none")]
    pub user_sign: i64,
    #[serde(rename = "TransNum", skip_serializing_if = "Option::is_none")]
    pub trans_num: Value,
    #[serde(rename = "VatSum", skip_serializing_if = "Option::is_none")]
    pub vat_sum: f64,
    #[serde(rename = "VatSumSys", skip_serializing_if = "Option::is_none")]
    pub vat_sum_sys: f64,
    #[serde(rename = "VatSumFc", skip_serializing_if = "Option::is_none")]
    pub vat_sum_fc: f64,
    #[serde(rename = "NetProcedure", skip_serializing_if = "Option::is_none")]
    pub net_procedure: String,
    #[serde(rename = "DocTotalFc", skip_serializing_if = "Option::is_none")]
    pub doc_total_fc: f64,
    #[serde(rename = "DocTotalSys", skip_serializing_if = "Option::is_none")]
    pub doc_total_sys: f64,
    #[serde(rename = "Form1099", skip_serializing_if = "Option::is_none")]
    pub form1099: Value,
    #[serde(rename = "Box1099", skip_serializing_if = "Option::is_none")]
    pub box1099: Value,
    #[serde(rename = "RevisionPo", skip_serializing_if = "Option::is_none")]
    pub revision_po: String,
    #[serde(rename = "RequriedDate", skip_serializing_if = "Option::is_none")]
    pub requried_date: String,
    #[serde(rename = "CancelDate", skip_serializing_if = "Option::is_none")]
    pub cancel_date: String,
    #[serde(rename = "BlockDunning", skip_serializing_if = "Option::is_none")]
    pub block_dunning: String,
    #[serde(rename = "Submitted", skip_serializing_if = "Option::is_none")]
    pub submitted: String,
    #[serde(rename = "Segment", skip_serializing_if = "Option::is_none")]
    pub segment: i64,
    #[serde(rename = "PickStatus", skip_serializing_if = "Option::is_none")]
    pub pick_status: String,
    #[serde(rename = "Pick", skip_serializing_if = "Option::is_none")]
    pub pick: String,
    #[serde(rename = "PaymentMethod", skip_serializing_if = "Option::is_none")]
    pub payment_method: String,
    #[serde(rename = "PaymentBlock", skip_serializing_if = "Option::is_none")]
    pub payment_block: String,
    #[serde(rename = "PaymentBlockEntry", skip_serializing_if = "Option::is_none")]
    pub payment_block_entry: Value,
    #[serde(rename = "CentralBankIndicator", skip_serializing_if = "Option::is_none")]
    pub central_bank_indicator: Value,
    #[serde(rename = "MaximumCashDiscount", skip_serializing_if = "Option::is_none")]
    pub maximum_cash_discount: String,
    #[serde(rename = "Reserve", skip_serializing_if = "Option::is_none")]
    pub reserve: String,
    #[serde(rename = "Project", skip_serializing_if = "Option::is_none")]
    pub project: Value,
    #[serde(rename = "ExemptionValidityDateFrom", skip_serializing_if = "Option::is_none")]
    pub exemption_validity_date_from: Value,
    #[serde(rename = "ExemptionValidityDateTo", skip_serializing_if = "Option::is_none")]
    pub exemption_validity_date_to: Value,
    #[serde(rename = "WareHouseUpdateType", skip_serializing_if = "Option::is_none")]
    pub ware_house_update_type: String,
    #[serde(rename = "Rounding", skip_serializing_if = "Option::is_none")]
    pub rounding: String,
    #[serde(rename = "ExternalCorrectedDocNum", skip_serializing_if = "Option::is_none")]
    pub external_corrected_doc_num: Value,
    #[serde(rename = "InternalCorrectedDocNum", skip_serializing_if = "Option::is_none")]
    pub internal_corrected_doc_num: Value,
    #[serde(rename = "NextCorrectingDocument", skip_serializing_if = "Option::is_none")]
    pub next_correcting_document: Value,
    #[serde(rename = "DeferredTax", skip_serializing_if = "Option::is_none")]
    pub deferred_tax: String,
    #[serde(rename = "TaxExemptionLetterNum", skip_serializing_if = "Option::is_none")]
    pub tax_exemption_letter_num: String,
    #[serde(rename = "WTApplied", skip_serializing_if = "Option::is_none")]
    pub wtapplied: f64,
    #[serde(rename = "WTAppliedFC", skip_serializing_if = "Option::is_none")]
    pub wtapplied_fc: f64,
    #[serde(rename = "BillOfExchangeReserved", skip_serializing_if = "Option::is_none")]
    pub bill_of_exchange_reserved: String,
    #[serde(rename = "AgentCode", skip_serializing_if = "Option::is_none")]
    pub agent_code: Value,
    #[serde(rename = "WTAppliedSC", skip_serializing_if = "Option::is_none")]
    pub wtapplied_sc: f64,
    #[serde(rename = "TotalEqualizationTax", skip_serializing_if = "Option::is_none")]
    pub total_equalization_tax: f64,
    #[serde(rename = "TotalEqualizationTaxFC", skip_serializing_if = "Option::is_none")]
    pub total_equalization_tax_fc: f64,
    #[serde(rename = "TotalEqualizationTaxSC", skip_serializing_if = "Option::is_none")]
    pub total_equalization_tax_sc: f64,
    #[serde(rename = "NumberOfInstallments", skip_serializing_if = "Option::is_none")]
    pub number_of_installments: i64,
    #[serde(rename = "ApplyTaxOnFirstInstallment", skip_serializing_if = "Option::is_none")]
    pub apply_tax_on_first_installment: String,
    #[serde(rename = "WTNonSubjectAmount", skip_serializing_if = "Option::is_none")]
    pub wtnon_subject_amount: f64,
    #[serde(rename = "WTNonSubjectAmountSC", skip_serializing_if = "Option::is_none")]
    pub wtnon_subject_amount_sc: f64,
    #[serde(rename = "WTNonSubjectAmountFC", skip_serializing_if = "Option::is_none")]
    pub wtnon_subject_amount_fc: f64,
    #[serde(rename = "WTExemptedAmount", skip_serializing_if = "Option::is_none")]
    pub wtexempted_amount: f64,
    #[serde(rename = "WTExemptedAmountSC", skip_serializing_if = "Option::is_none")]
    pub wtexempted_amount_sc: f64,
    #[serde(rename = "WTExemptedAmountFC", skip_serializing_if = "Option::is_none")]
    pub wtexempted_amount_fc: f64,
    #[serde(rename = "BaseAmount", skip_serializing_if = "Option::is_none")]
    pub base_amount: f64,
    #[serde(rename = "BaseAmountSC", skip_serializing_if = "Option::is_none")]
    pub base_amount_sc: f64,
    #[serde(rename = "BaseAmountFC", skip_serializing_if = "Option::is_none")]
    pub base_amount_fc: f64,
    #[serde(rename = "WTAmount", skip_serializing_if = "Option::is_none")]
    pub wtamount: f64,
    #[serde(rename = "WTAmountSC", skip_serializing_if = "Option::is_none")]
    pub wtamount_sc: f64,
    #[serde(rename = "WTAmountFC", skip_serializing_if = "Option::is_none")]
    pub wtamount_fc: f64,
    #[serde(rename = "VatDate", skip_serializing_if = "Option::is_none")]
    pub vat_date: Value,
    #[serde(rename = "DocumentsOwner", skip_serializing_if = "Option::is_none")]
    pub documents_owner: Value,
    #[serde(rename = "FolioPrefixString", skip_serializing_if = "Option::is_none")]
    pub folio_prefix_string: Value,
    #[serde(rename = "FolioNumber", skip_serializing_if = "Option::is_none")]
    pub folio_number: Value,
    #[serde(rename = "DocumentSubType", skip_serializing_if = "Option::is_none")]
    pub document_sub_type: String,
    #[serde(rename = "BPChannelCode", skip_serializing_if = "Option::is_none")]
    pub bpchannel_code: Value,
    #[serde(rename = "BPChannelContact", skip_serializing_if = "Option::is_none")]
    pub bpchannel_contact: i64,
    #[serde(rename = "Address2", skip_serializing_if = "Option::is_none")]
    pub address2: String,
    #[serde(rename = "DocumentStatus", skip_serializing_if = "Option::is_none")]
    pub document_status: String,
    #[serde(rename = "PeriodIndicator", skip_serializing_if = "Option::is_none")]
    pub period_indicator: String,
    #[serde(rename = "PayToCode", skip_serializing_if = "Option::is_none")]
    pub pay_to_code: String,
    #[serde(rename = "ManualNumber", skip_serializing_if = "Option::is_none")]
    pub manual_number: Value,
    #[serde(rename = "UseShpdGoodsAct", skip_serializing_if = "Option::is_none")]
    pub use_shpd_goods_act: String,
    #[serde(rename = "IsPayToBank", skip_serializing_if = "Option::is_none")]
    pub is_pay_to_bank: String,
    #[serde(rename = "PayToBankCountry", skip_serializing_if = "Option::is_none")]
    pub pay_to_bank_country: Value,
    #[serde(rename = "PayToBankCode", skip_serializing_if = "Option::is_none")]
    pub pay_to_bank_code: Value,
    #[serde(rename = "PayToBankAccountNo", skip_serializing_if = "Option::is_none")]
    pub pay_to_bank_account_no: String,
    #[serde(rename = "PayToBankBranch", skip_serializing_if = "Option::is_none")]
    pub pay_to_bank_branch: Value,
    #[serde(rename = "BPL_IDAssignedToInvoice", skip_serializing_if = "Option::is_none")]
    pub bpl_idassigned_to_invoice: i64,
    #[serde(rename = "DownPayment", skip_serializing_if = "Option::is_none")]
    pub down_payment: f64,
    #[serde(rename = "ReserveInvoice", skip_serializing_if = "Option::is_none")]
    pub reserve_invoice: String,
    #[serde(rename = "LanguageCode", skip_serializing_if = "Option::is_none")]
    pub language_code: i64,
    #[serde(rename = "TrackingNumber", skip_serializing_if = "Option::is_none")]
    pub tracking_number: Value,
    #[serde(rename = "PickRemark", skip_serializing_if = "Option::is_none")]
    pub pick_remark: Value,
    #[serde(rename = "ClosingDate", skip_serializing_if = "Option::is_none")]
    pub closing_date: Value,
    #[serde(rename = "SequenceCode", skip_serializing_if = "Option::is_none")]
    pub sequence_code: Value,
    #[serde(rename = "SequenceSerial", skip_serializing_if = "Option::is_none")]
    pub sequence_serial: Value,
    #[serde(rename = "SeriesString", skip_serializing_if = "Option::is_none")]
    pub series_string: Value,
    #[serde(rename = "SubSeriesString", skip_serializing_if = "Option::is_none")]
    pub sub_series_string: Value,
    #[serde(rename = "SequenceModel", skip_serializing_if = "Option::is_none")]
    pub sequence_model: String,
    #[serde(rename = "UseCorrectionVATGroup", skip_serializing_if = "Option::is_none")]
    pub use_correction_vatgroup: String,
    #[serde(rename = "TotalDiscount", skip_serializing_if = "Option::is_none")]
    pub total_discount: f64,
    #[serde(rename = "DownPaymentAmount", skip_serializing_if = "Option::is_none")]
    pub down_payment_amount: f64,
    #[serde(rename = "DownPaymentPercentage", skip_serializing_if = "Option::is_none")]
    pub down_payment_percentage: f64,
    #[serde(rename = "DownPaymentType", skip_serializing_if = "Option::is_none")]
    pub down_payment_type: String,
    #[serde(rename = "DownPaymentAmountSC", skip_serializing_if = "Option::is_none")]
    pub down_payment_amount_sc: f64,
    #[serde(rename = "DownPaymentAmountFC", skip_serializing_if = "Option::is_none")]
    pub down_payment_amount_fc: f64,
    #[serde(rename = "VatPercent", skip_serializing_if = "Option::is_none")]
    pub vat_percent: f64,
    #[serde(rename = "ServiceGrossProfitPercent", skip_serializing_if = "Option::is_none")]
    pub service_gross_profit_percent: f64,
    #[serde(rename = "OpeningRemarks", skip_serializing_if = "Option::is_none")]
    pub opening_remarks: Value,
    #[serde(rename = "ClosingRemarks", skip_serializing_if = "Option::is_none")]
    pub closing_remarks: Value,
    #[serde(rename = "RoundingDiffAmount", skip_serializing_if = "Option::is_none")]
    pub rounding_diff_amount: f64,
    #[serde(rename = "RoundingDiffAmountFC", skip_serializing_if = "Option::is_none")]
    pub rounding_diff_amount_fc: f64,
    #[serde(rename = "RoundingDiffAmountSC", skip_serializing_if = "Option::is_none")]
    pub rounding_diff_amount_sc: f64,
    #[serde(rename = "Cancelled", skip_serializing_if = "Option::is_none")]
    pub cancelled: String,
    #[serde(rename = "SignatureInputMessage", skip_serializing_if = "Option::is_none")]
    pub signature_input_message: Value,
    #[serde(rename = "SignatureDigest", skip_serializing_if = "Option::is_none")]
    pub signature_digest: Value,
    #[serde(rename = "CertificationNumber", skip_serializing_if = "Option::is_none")]
    pub certification_number: Value,
    #[serde(rename = "PrivateKeyVersion", skip_serializing_if = "Option::is_none")]
    pub private_key_version: Value,
    #[serde(rename = "ControlAccount", skip_serializing_if = "Option::is_none")]
    pub control_account: String,
    #[serde(rename = "InsuranceOperation347", skip_serializing_if = "Option::is_none")]
    pub insurance_operation347: String,
    #[serde(rename = "ArchiveNonremovableSalesQuotation", skip_serializing_if = "Option::is_none")]
    pub archive_nonremovable_sales_quotation: String,
    #[serde(rename = "GTSChecker", skip_serializing_if = "Option::is_none")]
    pub gtschecker: Value,
    #[serde(rename = "GTSPayee", skip_serializing_if = "Option::is_none")]
    pub gtspayee: Value,
    #[serde(rename = "ExtraMonth", skip_serializing_if = "Option::is_none")]
    pub extra_month: i64,
    #[serde(rename = "ExtraDays", skip_serializing_if = "Option::is_none")]
    pub extra_days: i64,
    #[serde(rename = "CashDiscountDateOffset", skip_serializing_if = "Option::is_none")]
    pub cash_discount_date_offset: i64,
    #[serde(rename = "StartFrom", skip_serializing_if = "Option::is_none")]
    pub start_from: String,
    #[serde(rename = "NTSApproved", skip_serializing_if = "Option::is_none")]
    pub ntsapproved: String,
    #[serde(rename = "ETaxWebSite", skip_serializing_if = "Option::is_none")]
    pub etax_web_site: Value,
    #[serde(rename = "ETaxNumber", skip_serializing_if = "Option::is_none")]
    pub etax_number: Value,
    #[serde(rename = "NTSApprovedNumber", skip_serializing_if = "Option::is_none")]
    pub ntsapproved_number: Value,
    #[serde(rename = "EDocGenerationType", skip_serializing_if = "Option::is_none")]
    pub edoc_generation_type: String,
    #[serde(rename = "EDocSeries", skip_serializing_if = "Option::is_none")]
    pub edoc_series: Value,
    #[serde(rename = "EDocNum", skip_serializing_if = "Option::is_none")]
    pub edoc_num: Value,
    #[serde(rename = "EDocExportFormat", skip_serializing_if = "Option::is_none")]
    pub edoc_export_format: Value,
    #[serde(rename = "EDocStatus", skip_serializing_if = "Option::is_none")]
    pub edoc_status: String,
    #[serde(rename = "EDocErrorCode", skip_serializing_if = "Option::is_none")]
    pub edoc_error_code: Value,
    #[serde(rename = "EDocErrorMessage", skip_serializing_if = "Option::is_none")]
    pub edoc_error_message: Value,
    #[serde(rename = "DownPaymentStatus", skip_serializing_if = "Option::is_none")]
    pub down_payment_status: String,
    #[serde(rename = "GroupSeries", skip_serializing_if = "Option::is_none")]
    pub group_series: Value,
    #[serde(rename = "GroupNumber", skip_serializing_if = "Option::is_none")]
    pub group_number: Value,
    #[serde(rename = "GroupHandWritten", skip_serializing_if = "Option::is_none")]
    pub group_hand_written: String,
    #[serde(rename = "ReopenOriginalDocument", skip_serializing_if = "Option::is_none")]
    pub reopen_original_document: Value,
    #[serde(rename = "ReopenManuallyClosedOrCanceledDocument", skip_serializing_if = "Option::is_none")]
    pub reopen_manually_closed_or_canceled_document: Value,
    #[serde(rename = "CreateOnlineQuotation", skip_serializing_if = "Option::is_none")]
    pub create_online_quotation: String,
    #[serde(rename = "POSEquipmentNumber", skip_serializing_if = "Option::is_none")]
    pub posequipment_number: Value,
    #[serde(rename = "POSManufacturerSerialNumber", skip_serializing_if = "Option::is_none")]
    pub posmanufacturer_serial_number: Value,
    #[serde(rename = "POSCashierNumber", skip_serializing_if = "Option::is_none")]
    pub poscashier_number: Value,
    #[serde(rename = "ApplyCurrentVATRatesForDownPaymentsToDraw", skip_serializing_if = "Option::is_none")]
    pub apply_current_vatrates_for_down_payments_to_draw: String,
    #[serde(rename = "ClosingOption", skip_serializing_if = "Option::is_none")]
    pub closing_option: String,
    #[serde(rename = "SpecifiedClosingDate", skip_serializing_if = "Option::is_none")]
    pub specified_closing_date: Value,
    #[serde(rename = "OpenForLandedCosts", skip_serializing_if = "Option::is_none")]
    pub open_for_landed_costs: String,
    #[serde(rename = "AuthorizationStatus", skip_serializing_if = "Option::is_none")]
    pub authorization_status: String,
    #[serde(rename = "TotalDiscountFC", skip_serializing_if = "Option::is_none")]
    pub total_discount_fc: f64,
    #[serde(rename = "TotalDiscountSC", skip_serializing_if = "Option::is_none")]
    pub total_discount_sc: f64,
    #[serde(rename = "RelevantToGTS", skip_serializing_if = "Option::is_none")]
    pub relevant_to_gts: String,
    #[serde(rename = "BPLName", skip_serializing_if = "Option::is_none")]
    pub bplname: String,
    #[serde(rename = "VATRegNum", skip_serializing_if = "Option::is_none")]
    pub vatreg_num: String,
    #[serde(rename = "AnnualInvoiceDeclarationReference", skip_serializing_if = "Option::is_none")]
    pub annual_invoice_declaration_reference: Value,
    #[serde(rename = "Supplier", skip_serializing_if = "Option::is_none")]
    pub supplier: Value,
    #[serde(rename = "Releaser", skip_serializing_if = "Option::is_none")]
    pub releaser: Value,
    #[serde(rename = "Receiver", skip_serializing_if = "Option::is_none")]
    pub receiver: Value,
    #[serde(rename = "BlanketAgreementNumber", skip_serializing_if = "Option::is_none")]
    pub blanket_agreement_number: Value,
    #[serde(rename = "IsAlteration", skip_serializing_if = "Option::is_none")]
    pub is_alteration: String,
    #[serde(rename = "CancelStatus", skip_serializing_if = "Option::is_none")]
    pub cancel_status: String,
    #[serde(rename = "AssetValueDate", skip_serializing_if = "Option::is_none")]
    pub asset_value_date: String,
    #[serde(rename = "DocumentDelivery", skip_serializing_if = "Option::is_none")]
    pub document_delivery: String,
    #[serde(rename = "AuthorizationCode", skip_serializing_if = "Option::is_none")]
    pub authorization_code: Value,
    #[serde(rename = "StartDeliveryDate", skip_serializing_if = "Option::is_none")]
    pub start_delivery_date: String,
    #[serde(rename = "StartDeliveryTime", skip_serializing_if = "Option::is_none")]
    pub start_delivery_time: Value,
    #[serde(rename = "EndDeliveryDate", skip_serializing_if = "Option::is_none")]
    pub end_delivery_date: String,
    #[serde(rename = "EndDeliveryTime", skip_serializing_if = "Option::is_none")]
    pub end_delivery_time: Value,
    #[serde(rename = "VehiclePlate", skip_serializing_if = "Option::is_none")]
    pub vehicle_plate: Value,
    #[serde(rename = "ATDocumentType", skip_serializing_if = "Option::is_none")]
    pub atdocument_type: Value,
    #[serde(rename = "ElecCommStatus", skip_serializing_if = "Option::is_none")]
    pub elec_comm_status: Value,
    #[serde(rename = "ElecCommMessage", skip_serializing_if = "Option::is_none")]
    pub elec_comm_message: Value,
    #[serde(rename = "ReuseDocumentNum", skip_serializing_if = "Option::is_none")]
    pub reuse_document_num: String,
    #[serde(rename = "ReuseNotaFiscalNum", skip_serializing_if = "Option::is_none")]
    pub reuse_nota_fiscal_num: String,
    #[serde(rename = "PrintSEPADirect", skip_serializing_if = "Option::is_none")]
    pub print_sepadirect: String,
    #[serde(rename = "FiscalDocNum", skip_serializing_if = "Option::is_none")]
    pub fiscal_doc_num: Value,
    #[serde(rename = "POSDailySummaryNo", skip_serializing_if = "Option::is_none")]
    pub posdaily_summary_no: Value,
    #[serde(rename = "POSReceiptNo", skip_serializing_if = "Option::is_none")]
    pub posreceipt_no: Value,
    #[serde(rename = "PointOfIssueCode", skip_serializing_if = "Option::is_none")]
    pub point_of_issue_code: Value,
    #[serde(rename = "Letter", skip_serializing_if = "Option::is_none")]
    pub letter: Value,
    #[serde(rename = "FolioNumberFrom", skip_serializing_if = "Option::is_none")]
    pub folio_number_from: Value,
    #[serde(rename = "FolioNumberTo", skip_serializing_if = "Option::is_none")]
    pub folio_number_to: Value,
    #[serde(rename = "InterimType", skip_serializing_if = "Option::is_none")]
    pub interim_type: String,
    #[serde(rename = "RelatedType", skip_serializing_if = "Option::is_none")]
    pub related_type: i64,
    #[serde(rename = "RelatedEntry", skip_serializing_if = "Option::is_none")]
    pub related_entry: Value,
    #[serde(rename = "SAPPassport", skip_serializing_if = "Option::is_none")]
    pub sappassport: Value,
    #[serde(rename = "DocumentTaxID", skip_serializing_if = "Option::is_none")]
    pub document_tax_id: Value,
    #[serde(rename = "DateOfReportingControlStatementVAT", skip_serializing_if = "Option::is_none")]
    pub date_of_reporting_control_statement_vat: Value,
    #[serde(rename = "ReportingSectionControlStatementVAT", skip_serializing_if = "Option::is_none")]
    pub reporting_section_control_statement_vat: Value,
    #[serde(rename = "ExcludeFromTaxReportControlStatementVAT", skip_serializing_if = "Option::is_none")]
    pub exclude_from_tax_report_control_statement_vat: String,
    #[serde(rename = "POS_CashRegister", skip_serializing_if = "Option::is_none")]
    pub pos_cash_register: Value,
    #[serde(rename = "UpdateTime", skip_serializing_if = "Option::is_none")]
    pub update_time: String,
    #[serde(rename = "CreateQRCodeFrom", skip_serializing_if = "Option::is_none")]
    pub create_qrcode_from: Value,
    #[serde(rename = "PriceMode", skip_serializing_if = "Option::is_none")]
    pub price_mode: Value,
    #[serde(rename = "ShipFrom", skip_serializing_if = "Option::is_none")]
    pub ship_from: String,
    #[serde(rename = "CommissionTrade", skip_serializing_if = "Option::is_none")]
    pub commission_trade: String,
    #[serde(rename = "CommissionTradeReturn", skip_serializing_if = "Option::is_none")]
    pub commission_trade_return: String,
    #[serde(rename = "UseBillToAddrToDetermineTax", skip_serializing_if = "Option::is_none")]
    pub use_bill_to_addr_to_determine_tax: String,
    #[serde(rename = "Cig", skip_serializing_if = "Option::is_none")]
    pub cig: Value,
    #[serde(rename = "Cup", skip_serializing_if = "Option::is_none")]
    pub cup: Value,
    #[serde(rename = "FatherCard", skip_serializing_if = "Option::is_none")]
    pub father_card: Value,
    #[serde(rename = "FatherType", skip_serializing_if = "Option::is_none")]
    pub father_type: String,
    #[serde(rename = "ShipState", skip_serializing_if = "Option::is_none")]
    pub ship_state: Value,
    #[serde(rename = "ShipPlace", skip_serializing_if = "Option::is_none")]
    pub ship_place: Value,
    #[serde(rename = "CustOffice", skip_serializing_if = "Option::is_none")]
    pub cust_office: Value,
    #[serde(rename = "FCI", skip_serializing_if = "Option::is_none")]
    pub fci: Value,
    #[serde(rename = "AddLegIn", skip_serializing_if = "Option::is_none")]
    pub add_leg_in: Value,
    #[serde(rename = "LegTextF", skip_serializing_if = "Option::is_none")]
    pub leg_text_f: Value,
    #[serde(rename = "DANFELgTxt", skip_serializing_if = "Option::is_none")]
    pub danfelg_txt: Value,
    #[serde(rename = "IndFinal", skip_serializing_if = "Option::is_none")]
    pub ind_final: String,
    #[serde(rename = "DataVersion", skip_serializing_if = "Option::is_none")]
    pub data_version: i64,
    #[serde(rename = "LastPageFolioNumber", skip_serializing_if = "Option::is_none")]
    pub last_page_folio_number: Value,
    #[serde(rename = "InventoryStatus", skip_serializing_if = "Option::is_none")]
    pub inventory_status: String,
    #[serde(rename = "PlasticPackagingTaxRelevant", skip_serializing_if = "Option::is_none")]
    pub plastic_packaging_tax_relevant: String,
    #[serde(rename = "U_ACW_Shipmark", skip_serializing_if = "Option::is_none")]
    pub u_acw_shipmark: Value,
    #[serde(rename = "U_ACW_From", skip_serializing_if = "Option::is_none")]
    pub u_acw_from: Value,
    #[serde(rename = "U_ACW_VIA", skip_serializing_if = "Option::is_none")]
    pub u_acw_via: Value,
    #[serde(rename = "U_ACW_To", skip_serializing_if = "Option::is_none")]
    pub u_acw_to: Value,
    #[serde(rename = "U_ACW_Destination", skip_serializing_if = "Option::is_none")]
    pub u_acw_destination: Value,
    #[serde(rename = "U_TBD_Shipment", skip_serializing_if = "Option::is_none")]
    pub u_tbd_shipment: Value,
    #[serde(rename = "U_TBD_Sales_Terms", skip_serializing_if = "Option::is_none")]
    pub u_tbd_sales_terms: Value,
    #[serde(rename = "U_TBD_Ship_on", skip_serializing_if = "Option::is_none")]
    pub u_tbd_ship_on: Value,
    #[serde(rename = "U_TBD_SC_Remarks", skip_serializing_if = "Option::is_none")]
    pub u_tbd_sc_remarks: Value,
    #[serde(rename = "U_TBD_SI_Remarks", skip_serializing_if = "Option::is_none")]
    pub u_tbd_si_remarks: Value,
    #[serde(rename = "U_TBD_PO_Remarks", skip_serializing_if = "Option::is_none")]
    pub u_tbd_po_remarks: Value,
    #[serde(rename = "U_TBD_Del_Remarks", skip_serializing_if = "Option::is_none")]
    pub u_tbd_del_remarks: Value,
    #[serde(rename = "U_TBD_SA_Remarks", skip_serializing_if = "Option::is_none")]
    pub u_tbd_sa_remarks: String,
    #[serde(rename = "U_TBD_Ver_SC", skip_serializing_if = "Option::is_none")]
    pub u_tbd_ver_sc: Value,
    #[serde(rename = "U_TBD_Ver_PO", skip_serializing_if = "Option::is_none")]
    pub u_tbd_ver_po: Value,
    #[serde(rename = "U_TBD_Ver_PL", skip_serializing_if = "Option::is_none")]
    pub u_tbd_ver_pl: Value,
    #[serde(rename = "U_TBD_Ver_SI", skip_serializing_if = "Option::is_none")]
    pub u_tbd_ver_si: Value,
    #[serde(rename = "U_TBD_FREIGHT", skip_serializing_if = "Option::is_none")]
    pub u_tbd_freight: Value,
    #[serde(rename = "U_PONum", skip_serializing_if = "Option::is_none")]
    pub u_ponum: Value,
    #[serde(rename = "U_OldSONum", skip_serializing_if = "Option::is_none")]
    pub u_old_sonum: Value,
    #[serde(rename = "U_OldPONum", skip_serializing_if = "Option::is_none")]
    pub u_old_ponum: Value,
    #[serde(rename = "U_InvoiceNum", skip_serializing_if = "Option::is_none")]
    pub u_invoice_num: Value,
    #[serde(rename = "U_V33_sonum", skip_serializing_if = "Option::is_none")]
    pub u_v33_sonum: Value,
    #[serde(rename = "U_ASN_Number", skip_serializing_if = "Option::is_none")]
    pub u_asn_number: Value,
    #[serde(rename = "U_WHBatch", skip_serializing_if = "Option::is_none")]
    pub u_whbatch: Value,
    #[serde(rename = "U_ChargebackType", skip_serializing_if = "Option::is_none")]
    pub u_chargeback_type: Value,
    #[serde(rename = "U_CharegbackStatus", skip_serializing_if = "Option::is_none")]
    pub u_charegback_status: Value,
    #[serde(rename = "U_ChargebackDesc", skip_serializing_if = "Option::is_none")]
    pub u_chargeback_desc: Value,
    #[serde(rename = "U_ChargebackDept", skip_serializing_if = "Option::is_none")]
    pub u_chargeback_dept: Value,
    #[serde(rename = "U_U_CMCHECK", skip_serializing_if = "Option::is_none")]
    pub u_u_cmcheck: String,
    #[serde(rename = "U_TBD_EDI_CTRL", skip_serializing_if = "Option::is_none")]
    pub u_tbd_edi_ctrl: String,
    #[serde(rename = "U_U_WSO", skip_serializing_if = "Option::is_none")]
    pub u_u_wso: String,
    #[serde(rename = "U_BOY_E0_CCPAYEX", skip_serializing_if = "Option::is_none")]
    pub u_boy_e0_ccpayex: String,
    #[serde(rename = "U_BOY_E0_AUTHACTION", skip_serializing_if = "Option::is_none")]
    pub u_boy_e0_authaction: String,
    #[serde(rename = "U_BOY_E0_ONETIMCC", skip_serializing_if = "Option::is_none")]
    pub u_boy_e0_onetimcc: String,
    #[serde(rename = "U_BeginWindowDate", skip_serializing_if = "Option::is_none")]
    pub u_begin_window_date: String,
    #[serde(rename = "U_EndWindowDate", skip_serializing_if = "Option::is_none")]
    pub u_end_window_date: String,
    #[serde(rename = "U_OrderWindowDate", skip_serializing_if = "Option::is_none")]
    pub u_order_window_date: Value,
    #[serde(rename = "U_BillingType", skip_serializing_if = "Option::is_none")]
    pub u_billing_type: String,
    #[serde(rename = "U_AccountZip", skip_serializing_if = "Option::is_none")]
    pub u_account_zip: String,
    #[serde(rename = "U_ExcludeEmail", skip_serializing_if = "Option::is_none")]
    pub u_exclude_email: String,
    #[serde(rename = "U_TBD_Amount_BL", skip_serializing_if = "Option::is_none")]
    pub u_tbd_amount_bl: Value,
    #[serde(rename = "U_TBD_Ship_Address", skip_serializing_if = "Option::is_none")]
    pub u_tbd_ship_address: Value,
    #[serde(rename = "U_TBD_TotalPage", skip_serializing_if = "Option::is_none")]
    pub u_tbd_total_page: Value,
    #[serde(rename = "U_PymtMatch", skip_serializing_if = "Option::is_none")]
    pub u_pymt_match: Value,
    #[serde(rename = "U_ExportingCarrier", skip_serializing_if = "Option::is_none")]
    pub u_exporting_carrier: Value,
    #[serde(rename = "U_LoadPierTerminal", skip_serializing_if = "Option::is_none")]
    pub u_load_pier_terminal: Value,
    #[serde(rename = "U_ExporterCntctName", skip_serializing_if = "Option::is_none")]
    pub u_exporter_cntct_name: Value,
    #[serde(rename = "U_ExporterCntctPhone", skip_serializing_if = "Option::is_none")]
    pub u_exporter_cntct_phone: Value,
    #[serde(rename = "U_Broker", skip_serializing_if = "Option::is_none")]
    pub u_broker: Value,
    #[serde(rename = "U_JE_Number", skip_serializing_if = "Option::is_none")]
    pub u_je_number: Value,
    #[serde(rename = "U_U_Department", skip_serializing_if = "Option::is_none")]
    pub u_u_department: Value,
    #[serde(rename = "U_TBD_TOTAL_CBM_ADJ", skip_serializing_if = "Option::is_none")]
    pub u_tbd_total_cbm_adj: f64,
    #[serde(rename = "U_TBD_TOTAL_GW_ADJ", skip_serializing_if = "Option::is_none")]
    pub u_tbd_total_gw_adj: f64,
    #[serde(rename = "U_ECSB1_BREX_Exchanged", skip_serializing_if = "Option::is_none")]
    pub u_ecsb1_brex_exchanged: String,
    #[serde(rename = "U_ECSB1_BREX_Branch", skip_serializing_if = "Option::is_none")]
    pub u_ecsb1_brex_branch: i64,
    #[serde(rename = "U_ECSB1_BREX_Processed", skip_serializing_if = "Option::is_none")]
    pub u_ecsb1_brex_processed: Value,
    #[serde(rename = "U_ECSB1_BREX_Reversed", skip_serializing_if = "Option::is_none")]
    pub u_ecsb1_brex_reversed: Value,
    #[serde(rename = "U_ECSB1_BREX_Target", skip_serializing_if = "Option::is_none")]
    pub u_ecsb1_brex_target: Value,
    #[serde(rename = "U_SHIP_SCAC", skip_serializing_if = "Option::is_none")]
    pub u_ship_scac: String,
    #[serde(rename = "U_SHIP_VIA", skip_serializing_if = "Option::is_none")]
    pub u_ship_via: String,
    #[serde(rename = "U_SHIP_VIA_DESC", skip_serializing_if = "Option::is_none")]
    pub u_ship_via_desc: String,
    #[serde(rename = "U_SHIP_VIA_ACCT", skip_serializing_if = "Option::is_none")]
    pub u_ship_via_acct: String,
    #[serde(rename = "U_SHIP_INTERSTAT", skip_serializing_if = "Option::is_none")]
    pub u_ship_interstat: String,
    #[serde(rename = "U_SHIP_REMARKS", skip_serializing_if = "Option::is_none")]
    pub u_ship_remarks: String,
    #[serde(rename = "U_IntConsignee", skip_serializing_if = "Option::is_none")]
    pub u_int_consignee: Value,
    #[serde(rename = "U_TBD_CARGO_RECEIVED_DATE", skip_serializing_if = "Option::is_none")]
    pub u_tbd_cargo_received_date: Value,
    #[serde(rename = "U_OQ_REF_VALUE", skip_serializing_if = "Option::is_none")]
    pub u_oq_ref_value: String,
    #[serde(rename = "U_TBD_OINum", skip_serializing_if = "Option::is_none")]
    pub u_tbd_oinum: Value,
    #[serde(rename = "U_CreditHold", skip_serializing_if = "Option::is_none")]
    pub u_credit_hold: String,
    #[serde(rename = "U_B2BOrderID", skip_serializing_if = "Option::is_none")]
    pub u_b2border_id: Value,
    #[serde(rename = "U_TBD_Container_No", skip_serializing_if = "Option::is_none")]
    pub u_tbd_container_no: Value,
    #[serde(rename = "U_TBD_SO_Ref", skip_serializing_if = "Option::is_none")]
    pub u_tbd_so_ref: Value,
    #[serde(rename = "U_TBD_Cust_Name", skip_serializing_if = "Option::is_none")]
    pub u_tbd_cust_name: Value,
    #[serde(rename = "U_U_TBD_SO_No", skip_serializing_if = "Option::is_none")]
    pub u_u_tbd_so_no: Value,
    #[serde(rename = "U_ETD", skip_serializing_if = "Option::is_none")]
    pub u_etd: Value,
    #[serde(rename = "U_ETA", skip_serializing_if = "Option::is_none")]
    pub u_eta: Value,
    #[serde(rename = "U_MeijerPODC", skip_serializing_if = "Option::is_none")]
    pub u_meijer_podc: Value,
    #[serde(rename = "U_TransferBroker", skip_serializing_if = "Option::is_none")]
    pub u_transfer_broker: Value,
    #[serde(rename = "U_ShippingType", skip_serializing_if = "Option::is_none")]
    pub u_shipping_type: Value,
    #[serde(rename = "U_TransferReq", skip_serializing_if = "Option::is_none")]
    pub u_transfer_req: Value,
    #[serde(rename = "Document_ApprovalRequests", skip_serializing_if = "Option::is_none")]
    pub document_approval_requests: Vec<Value>,
    #[serde(rename = "DocumentLines", skip_serializing_if = "Option::is_none")]
    pub document_lines: Vec<DocumentLine>,
    #[serde(rename = "ElectronicProtocols", skip_serializing_if = "Option::is_none")]
    pub electronic_protocols: Vec<Value>,
    #[serde(rename = "DocumentAdditionalExpenses", skip_serializing_if = "Option::is_none")]
    pub document_additional_expenses: Vec<Value>,
    #[serde(rename = "WithholdingTaxDataWTXCollection", skip_serializing_if = "Option::is_none")]
    pub withholding_tax_data_wtxcollection: Vec<Value>,
    #[serde(rename = "WithholdingTaxDataCollection", skip_serializing_if = "Option::is_none")]
    pub withholding_tax_data_collection: Vec<Value>,
    #[serde(rename = "DocumentSpecialLines", skip_serializing_if = "Option::is_none")]
    pub document_special_lines: Vec<Value>,
    #[serde(rename = "TaxExtension", skip_serializing_if = "Option::is_none")]
    pub tax_extension: TaxExtension,
    #[serde(rename = "AddressExtension", skip_serializing_if = "Option::is_none")]
    pub address_extension: AddressExtension,
    #[serde(rename = "DocumentReferences", skip_serializing_if = "Option::is_none")]
    pub document_references: Vec<DocumentReference>,
}
*/

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentLine {
    #[serde(rename = "LineNum")]
    pub line_num: i32,
    #[serde(rename = "ItemCode")]
    pub item_code: String,
    #[serde(rename = "Quantity")]
    pub quantity: f32,
    #[serde(rename = "UnitPrice")]
    pub unit_price: f64,
    #[serde(rename = "SupplierCatNum", skip_serializing_if = "Option::is_none")]
    pub supplier_cat_num: Option<String>,
    #[serde(rename = "U_TBD_Cust_No", skip_serializing_if = "Option::is_none")]
    pub u_tbd_cust_no: Option<String>,
    #[serde(rename = "WarehouseCode")]
    pub warehouse_code: String,
    #[serde(rename = "TaxCode")]
    pub tax_code: Option<String>,
    #[serde(rename = "BarCode", skip_serializing_if = "Option::is_none")]
    pub bar_code: Option<String>,
    #[serde(rename = "U_ACW_DeliveryFrom", skip_serializing_if = "Option::is_none")]
    pub u_acw_deliverfrom: Option<String>,
    #[serde(rename = "U_ACW_DeliveryEnd", skip_serializing_if = "Option::is_none")]
    pub u_acw_deliveryend: Option<String>,
}
/*
pub struct DocumentLine {
    #[serde(rename = "LineNum", skip_serializing_if = "Option::is_none")]
    pub line_num: i64,
    #[serde(rename = "ItemCode", skip_serializing_if = "Option::is_none")]
    pub item_code: String,
    #[serde(rename = "ItemDescription", skip_serializing_if = "Option::is_none")]
    pub item_description: String,
    #[serde(rename = "Quantity", skip_serializing_if = "Option::is_none")]
    pub quantity: f64,
    #[serde(rename = "ShipDate", skip_serializing_if = "Option::is_none")]
    pub ship_date: String,
    #[serde(rename = "Price", skip_serializing_if = "Option::is_none")]
    pub price: f64,
    #[serde(rename = "PriceAfterVAT", skip_serializing_if = "Option::is_none")]
    pub price_after_vat: f64,
    #[serde(rename = "Currency", skip_serializing_if = "Option::is_none")]
    pub currency: String,
    #[serde(rename = "Rate", skip_serializing_if = "Option::is_none")]
    pub rate: f64,
    #[serde(rename = "DiscountPercent", skip_serializing_if = "Option::is_none")]
    pub discount_percent: f64,
    #[serde(rename = "VendorNum", skip_serializing_if = "Option::is_none")]
    pub vendor_num: Value,
    #[serde(rename = "SerialNum", skip_serializing_if = "Option::is_none")]
    pub serial_num: String,
    #[serde(rename = "WarehouseCode", skip_serializing_if = "Option::is_none")]
    pub warehouse_code: String,
    #[serde(rename = "SalesPersonCode", skip_serializing_if = "Option::is_none")]
    pub sales_person_code: i64,
    #[serde(rename = "CommisionPercent", skip_serializing_if = "Option::is_none")]
    pub commision_percent: f64,
    #[serde(rename = "TreeType", skip_serializing_if = "Option::is_none")]
    pub tree_type: String,
    #[serde(rename = "AccountCode", skip_serializing_if = "Option::is_none")]
    pub account_code: String,
    #[serde(rename = "UseBaseUnits", skip_serializing_if = "Option::is_none")]
    pub use_base_units: String,
    #[serde(rename = "SupplierCatNum", skip_serializing_if = "Option::is_none")]
    pub supplier_cat_num: String,
    #[serde(rename = "CostingCode", skip_serializing_if = "Option::is_none")]
    pub costing_code: Value,
    #[serde(rename = "ProjectCode", skip_serializing_if = "Option::is_none")]
    pub project_code: String,
    #[serde(rename = "BarCode", skip_serializing_if = "Option::is_none")]
    pub bar_code: String,
    #[serde(rename = "VatGroup", skip_serializing_if = "Option::is_none")]
    pub vat_group: Value,
    #[serde(rename = "Height1", skip_serializing_if = "Option::is_none")]
    pub height1: f64,
    #[serde(rename = "Hight1Unit", skip_serializing_if = "Option::is_none")]
    pub hight1unit: i64,
    #[serde(rename = "Height2", skip_serializing_if = "Option::is_none")]
    pub height2: f64,
    #[serde(rename = "Height2Unit", skip_serializing_if = "Option::is_none")]
    pub height2unit: Value,
    #[serde(rename = "Lengh1", skip_serializing_if = "Option::is_none")]
    pub lengh1: f64,
    #[serde(rename = "Lengh1Unit", skip_serializing_if = "Option::is_none")]
    pub lengh1unit: i64,
    #[serde(rename = "Lengh2", skip_serializing_if = "Option::is_none")]
    pub lengh2: f64,
    #[serde(rename = "Lengh2Unit", skip_serializing_if = "Option::is_none")]
    pub lengh2unit: Value,
    #[serde(rename = "Weight1", skip_serializing_if = "Option::is_none")]
    pub weight1: f64,
    #[serde(rename = "Weight1Unit", skip_serializing_if = "Option::is_none")]
    pub weight1unit: i64,
    #[serde(rename = "Weight2", skip_serializing_if = "Option::is_none")]
    pub weight2: f64,
    #[serde(rename = "Weight2Unit", skip_serializing_if = "Option::is_none")]
    pub weight2unit: Value,
    #[serde(rename = "Factor1", skip_serializing_if = "Option::is_none")]
    pub factor1: f64,
    #[serde(rename = "Factor2", skip_serializing_if = "Option::is_none")]
    pub factor2: f64,
    #[serde(rename = "Factor3", skip_serializing_if = "Option::is_none")]
    pub factor3: f64,
    #[serde(rename = "Factor4", skip_serializing_if = "Option::is_none")]
    pub factor4: f64,
    #[serde(rename = "BaseType", skip_serializing_if = "Option::is_none")]
    pub base_type: i64,
    #[serde(rename = "BaseEntry", skip_serializing_if = "Option::is_none")]
    pub base_entry: Value,
    #[serde(rename = "BaseLine", skip_serializing_if = "Option::is_none")]
    pub base_line: Value,
    #[serde(rename = "Volume", skip_serializing_if = "Option::is_none")]
    pub volume: f64,
    #[serde(rename = "VolumeUnit", skip_serializing_if = "Option::is_none")]
    pub volume_unit: i64,
    #[serde(rename = "Width1", skip_serializing_if = "Option::is_none")]
    pub width1: f64,
    #[serde(rename = "Width1Unit", skip_serializing_if = "Option::is_none")]
    pub width1unit: i64,
    #[serde(rename = "Width2", skip_serializing_if = "Option::is_none")]
    pub width2: f64,
    #[serde(rename = "Width2Unit", skip_serializing_if = "Option::is_none")]
    pub width2unit: Value,
    #[serde(rename = "Address", skip_serializing_if = "Option::is_none")]
    pub address: String,
    #[serde(rename = "TaxCode", skip_serializing_if = "Option::is_none")]
    pub tax_code: String,
    #[serde(rename = "TaxType", skip_serializing_if = "Option::is_none")]
    pub tax_type: String,
    #[serde(rename = "TaxLiable", skip_serializing_if = "Option::is_none")]
    pub tax_liable: String,
    #[serde(rename = "PickStatus", skip_serializing_if = "Option::is_none")]
    pub pick_status: String,
    #[serde(rename = "PickQuantity", skip_serializing_if = "Option::is_none")]
    pub pick_quantity: f64,
    #[serde(rename = "PickListIdNumber", skip_serializing_if = "Option::is_none")]
    pub pick_list_id_number: Value,
    #[serde(rename = "OriginalItem", skip_serializing_if = "Option::is_none")]
    pub original_item: Value,
    #[serde(rename = "BackOrder", skip_serializing_if = "Option::is_none")]
    pub back_order: String,
    #[serde(rename = "FreeText", skip_serializing_if = "Option::is_none")]
    pub free_text: Value,
    #[serde(rename = "ShippingMethod", skip_serializing_if = "Option::is_none")]
    pub shipping_method: i64,
    #[serde(rename = "POTargetNum", skip_serializing_if = "Option::is_none")]
    pub potarget_num: Value,
    #[serde(rename = "POTargetEntry", skip_serializing_if = "Option::is_none")]
    pub potarget_entry: String,
    #[serde(rename = "POTargetRowNum", skip_serializing_if = "Option::is_none")]
    pub potarget_row_num: Value,
    #[serde(rename = "CorrectionInvoiceItem", skip_serializing_if = "Option::is_none")]
    pub correction_invoice_item: String,
    #[serde(rename = "CorrInvAmountToStock", skip_serializing_if = "Option::is_none")]
    pub corr_inv_amount_to_stock: f64,
    #[serde(rename = "CorrInvAmountToDiffAcct", skip_serializing_if = "Option::is_none")]
    pub corr_inv_amount_to_diff_acct: f64,
    #[serde(rename = "AppliedTax", skip_serializing_if = "Option::is_none")]
    pub applied_tax: f64,
    #[serde(rename = "AppliedTaxFC", skip_serializing_if = "Option::is_none")]
    pub applied_tax_fc: f64,
    #[serde(rename = "AppliedTaxSC", skip_serializing_if = "Option::is_none")]
    pub applied_tax_sc: f64,
    #[serde(rename = "WTLiable", skip_serializing_if = "Option::is_none")]
    pub wtliable: String,
    #[serde(rename = "DeferredTax", skip_serializing_if = "Option::is_none")]
    pub deferred_tax: String,
    #[serde(rename = "EqualizationTaxPercent", skip_serializing_if = "Option::is_none")]
    pub equalization_tax_percent: f64,
    #[serde(rename = "TotalEqualizationTax", skip_serializing_if = "Option::is_none")]
    pub total_equalization_tax: f64,
    #[serde(rename = "TotalEqualizationTaxFC", skip_serializing_if = "Option::is_none")]
    pub total_equalization_tax_fc: f64,
    #[serde(rename = "TotalEqualizationTaxSC", skip_serializing_if = "Option::is_none")]
    pub total_equalization_tax_sc: f64,
    #[serde(rename = "NetTaxAmount", skip_serializing_if = "Option::is_none")]
    pub net_tax_amount: f64,
    #[serde(rename = "NetTaxAmountFC", skip_serializing_if = "Option::is_none")]
    pub net_tax_amount_fc: f64,
    #[serde(rename = "NetTaxAmountSC", skip_serializing_if = "Option::is_none")]
    pub net_tax_amount_sc: f64,
    #[serde(rename = "MeasureUnit", skip_serializing_if = "Option::is_none")]
    pub measure_unit: String,
    #[serde(rename = "UnitsOfMeasurment", skip_serializing_if = "Option::is_none")]
    pub units_of_measurment: f64,
    #[serde(rename = "LineTotal", skip_serializing_if = "Option::is_none")]
    pub line_total: f64,
    #[serde(rename = "TaxPercentagePerRow", skip_serializing_if = "Option::is_none")]
    pub tax_percentage_per_row: Value,
    #[serde(rename = "TaxTotal", skip_serializing_if = "Option::is_none")]
    pub tax_total: f64,
    #[serde(rename = "ConsumerSalesForecast", skip_serializing_if = "Option::is_none")]
    pub consumer_sales_forecast: String,
    #[serde(rename = "ExciseAmount", skip_serializing_if = "Option::is_none")]
    pub excise_amount: f64,
    #[serde(rename = "TaxPerUnit", skip_serializing_if = "Option::is_none")]
    pub tax_per_unit: f64,
    #[serde(rename = "TotalInclTax", skip_serializing_if = "Option::is_none")]
    pub total_incl_tax: f64,
    #[serde(rename = "CountryOrg", skip_serializing_if = "Option::is_none")]
    pub country_org: Value,
    #[serde(rename = "SWW", skip_serializing_if = "Option::is_none")]
    pub sww: Value,
    #[serde(rename = "TransactionType", skip_serializing_if = "Option::is_none")]
    pub transaction_type: Value,
    #[serde(rename = "DistributeExpense", skip_serializing_if = "Option::is_none")]
    pub distribute_expense: String,
    #[serde(rename = "RowTotalFC", skip_serializing_if = "Option::is_none")]
    pub row_total_fc: f64,
    #[serde(rename = "RowTotalSC", skip_serializing_if = "Option::is_none")]
    pub row_total_sc: f64,
    #[serde(rename = "LastBuyInmPrice", skip_serializing_if = "Option::is_none")]
    pub last_buy_inm_price: f64,
    #[serde(rename = "LastBuyDistributeSumFc", skip_serializing_if = "Option::is_none")]
    pub last_buy_distribute_sum_fc: f64,
    #[serde(rename = "LastBuyDistributeSumSc", skip_serializing_if = "Option::is_none")]
    pub last_buy_distribute_sum_sc: f64,
    #[serde(rename = "LastBuyDistributeSum", skip_serializing_if = "Option::is_none")]
    pub last_buy_distribute_sum: f64,
    #[serde(rename = "StockDistributesumForeign", skip_serializing_if = "Option::is_none")]
    pub stock_distributesum_foreign: f64,
    #[serde(rename = "StockDistributesumSystem", skip_serializing_if = "Option::is_none")]
    pub stock_distributesum_system: f64,
    #[serde(rename = "StockDistributesum", skip_serializing_if = "Option::is_none")]
    pub stock_distributesum: f64,
    #[serde(rename = "StockInmPrice", skip_serializing_if = "Option::is_none")]
    pub stock_inm_price: f64,
    #[serde(rename = "PickStatusEx", skip_serializing_if = "Option::is_none")]
    pub pick_status_ex: String,
    #[serde(rename = "TaxBeforeDPM", skip_serializing_if = "Option::is_none")]
    pub tax_before_dpm: f64,
    #[serde(rename = "TaxBeforeDPMFC", skip_serializing_if = "Option::is_none")]
    pub tax_before_dpmfc: f64,
    #[serde(rename = "TaxBeforeDPMSC", skip_serializing_if = "Option::is_none")]
    pub tax_before_dpmsc: f64,
    #[serde(rename = "CFOPCode", skip_serializing_if = "Option::is_none")]
    pub cfopcode: Value,
    #[serde(rename = "CSTCode", skip_serializing_if = "Option::is_none")]
    pub cstcode: Value,
    #[serde(rename = "Usage", skip_serializing_if = "Option::is_none")]
    pub usage: Value,
    #[serde(rename = "TaxOnly", skip_serializing_if = "Option::is_none")]
    pub tax_only: String,
    #[serde(rename = "VisualOrder", skip_serializing_if = "Option::is_none")]
    pub visual_order: i64,
    #[serde(rename = "BaseOpenQuantity", skip_serializing_if = "Option::is_none")]
    pub base_open_quantity: f64,
    #[serde(rename = "UnitPrice", skip_serializing_if = "Option::is_none")]
    pub unit_price: f64,
    #[serde(rename = "LineStatus", skip_serializing_if = "Option::is_none")]
    pub line_status: String,
    #[serde(rename = "PackageQuantity", skip_serializing_if = "Option::is_none")]
    pub package_quantity: f64,
    #[serde(rename = "Text", skip_serializing_if = "Option::is_none")]
    pub text: Value,
    #[serde(rename = "LineType", skip_serializing_if = "Option::is_none")]
    pub line_type: String,
    #[serde(rename = "COGSCostingCode", skip_serializing_if = "Option::is_none")]
    pub cogscosting_code: Value,
    #[serde(rename = "COGSAccountCode", skip_serializing_if = "Option::is_none")]
    pub cogsaccount_code: String,
    #[serde(rename = "ChangeAssemlyBoMWarehouse", skip_serializing_if = "Option::is_none")]
    pub change_assemly_bo_mwarehouse: String,
    #[serde(rename = "GrossBuyPrice", skip_serializing_if = "Option::is_none")]
    pub gross_buy_price: f64,
    #[serde(rename = "GrossBase", skip_serializing_if = "Option::is_none")]
    pub gross_base: i64,
    #[serde(rename = "GrossProfitTotalBasePrice", skip_serializing_if = "Option::is_none")]
    pub gross_profit_total_base_price: f64,
    #[serde(rename = "CostingCode2", skip_serializing_if = "Option::is_none")]
    pub costing_code2: Value,
    #[serde(rename = "CostingCode3", skip_serializing_if = "Option::is_none")]
    pub costing_code3: String,
    #[serde(rename = "CostingCode4", skip_serializing_if = "Option::is_none")]
    pub costing_code4: Value,
    #[serde(rename = "CostingCode5", skip_serializing_if = "Option::is_none")]
    pub costing_code5: Value,
    #[serde(rename = "ItemDetails", skip_serializing_if = "Option::is_none")]
    pub item_details: Value,
    #[serde(rename = "LocationCode", skip_serializing_if = "Option::is_none")]
    pub location_code: Value,
    #[serde(rename = "ActualDeliveryDate", skip_serializing_if = "Option::is_none")]
    pub actual_delivery_date: Value,
    #[serde(rename = "RemainingOpenQuantity", skip_serializing_if = "Option::is_none")]
    pub remaining_open_quantity: f64,
    #[serde(rename = "OpenAmount", skip_serializing_if = "Option::is_none")]
    pub open_amount: f64,
    #[serde(rename = "OpenAmountFC", skip_serializing_if = "Option::is_none")]
    pub open_amount_fc: f64,
    #[serde(rename = "OpenAmountSC", skip_serializing_if = "Option::is_none")]
    pub open_amount_sc: f64,
    #[serde(rename = "ExLineNo", skip_serializing_if = "Option::is_none")]
    pub ex_line_no: Value,
    #[serde(rename = "RequiredDate", skip_serializing_if = "Option::is_none")]
    pub required_date: Value,
    #[serde(rename = "RequiredQuantity", skip_serializing_if = "Option::is_none")]
    pub required_quantity: f64,
    #[serde(rename = "COGSCostingCode2", skip_serializing_if = "Option::is_none")]
    pub cogscosting_code2: Value,
    #[serde(rename = "COGSCostingCode3", skip_serializing_if = "Option::is_none")]
    pub cogscosting_code3: String,
    #[serde(rename = "COGSCostingCode4", skip_serializing_if = "Option::is_none")]
    pub cogscosting_code4: Value,
    #[serde(rename = "COGSCostingCode5", skip_serializing_if = "Option::is_none")]
    pub cogscosting_code5: Value,
    #[serde(rename = "CSTforIPI", skip_serializing_if = "Option::is_none")]
    pub cstfor_ipi: Value,
    #[serde(rename = "CSTforPIS", skip_serializing_if = "Option::is_none")]
    pub cstfor_pis: Value,
    #[serde(rename = "CSTforCOFINS", skip_serializing_if = "Option::is_none")]
    pub cstfor_cofins: Value,
    #[serde(rename = "CreditOriginCode", skip_serializing_if = "Option::is_none")]
    pub credit_origin_code: Value,
    #[serde(rename = "WithoutInventoryMovement", skip_serializing_if = "Option::is_none")]
    pub without_inventory_movement: String,
    #[serde(rename = "AgreementNo", skip_serializing_if = "Option::is_none")]
    pub agreement_no: Value,
    #[serde(rename = "AgreementRowNumber", skip_serializing_if = "Option::is_none")]
    pub agreement_row_number: Value,
    #[serde(rename = "ActualBaseEntry", skip_serializing_if = "Option::is_none")]
    pub actual_base_entry: Value,
    #[serde(rename = "ActualBaseLine", skip_serializing_if = "Option::is_none")]
    pub actual_base_line: Value,
    #[serde(rename = "DocEntry", skip_serializing_if = "Option::is_none")]
    pub doc_entry: i64,
    #[serde(rename = "Surpluses", skip_serializing_if = "Option::is_none")]
    pub surpluses: f64,
    #[serde(rename = "DefectAndBreakup", skip_serializing_if = "Option::is_none")]
    pub defect_and_breakup: f64,
    #[serde(rename = "Shortages", skip_serializing_if = "Option::is_none")]
    pub shortages: f64,
    #[serde(rename = "ConsiderQuantity", skip_serializing_if = "Option::is_none")]
    pub consider_quantity: String,
    #[serde(rename = "PartialRetirement", skip_serializing_if = "Option::is_none")]
    pub partial_retirement: String,
    #[serde(rename = "RetirementQuantity", skip_serializing_if = "Option::is_none")]
    pub retirement_quantity: f64,
    #[serde(rename = "RetirementAPC", skip_serializing_if = "Option::is_none")]
    pub retirement_apc: f64,
    #[serde(rename = "ThirdParty", skip_serializing_if = "Option::is_none")]
    pub third_party: String,
    #[serde(rename = "PoNum", skip_serializing_if = "Option::is_none")]
    pub po_num: Value,
    #[serde(rename = "PoItmNum", skip_serializing_if = "Option::is_none")]
    pub po_itm_num: Value,
    #[serde(rename = "ExpenseType", skip_serializing_if = "Option::is_none")]
    pub expense_type: Value,
    #[serde(rename = "ReceiptNumber", skip_serializing_if = "Option::is_none")]
    pub receipt_number: Value,
    #[serde(rename = "ExpenseOperationType", skip_serializing_if = "Option::is_none")]
    pub expense_operation_type: Value,
    #[serde(rename = "FederalTaxID", skip_serializing_if = "Option::is_none")]
    pub federal_tax_id: Value,
    #[serde(rename = "GrossProfit", skip_serializing_if = "Option::is_none")]
    pub gross_profit: f64,
    #[serde(rename = "GrossProfitFC", skip_serializing_if = "Option::is_none")]
    pub gross_profit_fc: f64,
    #[serde(rename = "GrossProfitSC", skip_serializing_if = "Option::is_none")]
    pub gross_profit_sc: f64,
    #[serde(rename = "PriceSource", skip_serializing_if = "Option::is_none")]
    pub price_source: String,
    #[serde(rename = "StgSeqNum", skip_serializing_if = "Option::is_none")]
    pub stg_seq_num: Value,
    #[serde(rename = "StgEntry", skip_serializing_if = "Option::is_none")]
    pub stg_entry: Value,
    #[serde(rename = "StgDesc", skip_serializing_if = "Option::is_none")]
    pub stg_desc: Value,
    #[serde(rename = "UoMEntry", skip_serializing_if = "Option::is_none")]
    pub uo_mentry: i64,
    #[serde(rename = "UoMCode", skip_serializing_if = "Option::is_none")]
    pub uo_mcode: String,
    #[serde(rename = "InventoryQuantity", skip_serializing_if = "Option::is_none")]
    pub inventory_quantity: f64,
    #[serde(rename = "RemainingOpenInventoryQuantity", skip_serializing_if = "Option::is_none")]
    pub remaining_open_inventory_quantity: f64,
    #[serde(rename = "ParentLineNum", skip_serializing_if = "Option::is_none")]
    pub parent_line_num: Value,
    #[serde(rename = "Incoterms", skip_serializing_if = "Option::is_none")]
    pub incoterms: i64,
    #[serde(rename = "TransportMode", skip_serializing_if = "Option::is_none")]
    pub transport_mode: i64,
    #[serde(rename = "NatureOfTransaction", skip_serializing_if = "Option::is_none")]
    pub nature_of_transaction: Value,
    #[serde(rename = "DestinationCountryForImport", skip_serializing_if = "Option::is_none")]
    pub destination_country_for_import: Value,
    #[serde(rename = "DestinationRegionForImport", skip_serializing_if = "Option::is_none")]
    pub destination_region_for_import: Value,
    #[serde(rename = "OriginCountryForExport", skip_serializing_if = "Option::is_none")]
    pub origin_country_for_export: Value,
    #[serde(rename = "OriginRegionForExport", skip_serializing_if = "Option::is_none")]
    pub origin_region_for_export: Value,
    #[serde(rename = "ItemType", skip_serializing_if = "Option::is_none")]
    pub item_type: String,
    #[serde(rename = "ChangeInventoryQuantityIndependently", skip_serializing_if = "Option::is_none")]
    pub change_inventory_quantity_independently: String,
    #[serde(rename = "FreeOfChargeBP", skip_serializing_if = "Option::is_none")]
    pub free_of_charge_bp: String,
    #[serde(rename = "SACEntry", skip_serializing_if = "Option::is_none")]
    pub sacentry: Value,
    #[serde(rename = "HSNEntry", skip_serializing_if = "Option::is_none")]
    pub hsnentry: Value,
    #[serde(rename = "GrossPrice", skip_serializing_if = "Option::is_none")]
    pub gross_price: f64,
    #[serde(rename = "GrossTotal", skip_serializing_if = "Option::is_none")]
    pub gross_total: f64,
    #[serde(rename = "GrossTotalFC", skip_serializing_if = "Option::is_none")]
    pub gross_total_fc: f64,
    #[serde(rename = "GrossTotalSC", skip_serializing_if = "Option::is_none")]
    pub gross_total_sc: f64,
    #[serde(rename = "NCMCode", skip_serializing_if = "Option::is_none")]
    pub ncmcode: i64,
    #[serde(rename = "NVECode", skip_serializing_if = "Option::is_none")]
    pub nvecode: Value,
    #[serde(rename = "IndEscala", skip_serializing_if = "Option::is_none")]
    pub ind_escala: String,
    #[serde(rename = "CtrSealQty", skip_serializing_if = "Option::is_none")]
    pub ctr_seal_qty: f64,
    #[serde(rename = "CNJPMan", skip_serializing_if = "Option::is_none")]
    pub cnjpman: Value,
    #[serde(rename = "CESTCode", skip_serializing_if = "Option::is_none")]
    pub cestcode: Value,
    #[serde(rename = "UFFiscalBenefitCode", skip_serializing_if = "Option::is_none")]
    pub uffiscal_benefit_code: Value,
    #[serde(rename = "ReverseCharge", skip_serializing_if = "Option::is_none")]
    pub reverse_charge: String,
    #[serde(rename = "ShipToCode", skip_serializing_if = "Option::is_none")]
    pub ship_to_code: String,
    #[serde(rename = "ShipToDescription", skip_serializing_if = "Option::is_none")]
    pub ship_to_description: String,
    #[serde(rename = "OwnerCode", skip_serializing_if = "Option::is_none")]
    pub owner_code: Value,
    #[serde(rename = "ExternalCalcTaxRate", skip_serializing_if = "Option::is_none")]
    pub external_calc_tax_rate: f64,
    #[serde(rename = "ExternalCalcTaxAmount", skip_serializing_if = "Option::is_none")]
    pub external_calc_tax_amount: f64,
    #[serde(rename = "ExternalCalcTaxAmountFC", skip_serializing_if = "Option::is_none")]
    pub external_calc_tax_amount_fc: f64,
    #[serde(rename = "ExternalCalcTaxAmountSC", skip_serializing_if = "Option::is_none")]
    pub external_calc_tax_amount_sc: f64,
    #[serde(rename = "StandardItemIdentification", skip_serializing_if = "Option::is_none")]
    pub standard_item_identification: i64,
    #[serde(rename = "CommodityClassification", skip_serializing_if = "Option::is_none")]
    pub commodity_classification: i64,
    #[serde(rename = "UnencumberedReason", skip_serializing_if = "Option::is_none")]
    pub unencumbered_reason: Value,
    #[serde(rename = "CUSplit", skip_serializing_if = "Option::is_none")]
    pub cusplit: String,
    #[serde(rename = "ListNum", skip_serializing_if = "Option::is_none")]
    pub list_num: i64,
    #[serde(rename = "RecognizedTaxCode", skip_serializing_if = "Option::is_none")]
    pub recognized_tax_code: Value,
    #[serde(rename = "U_ACW_POL", skip_serializing_if = "Option::is_none")]
    pub u_acw_pol: Value,
    #[serde(rename = "U_ACW_POD", skip_serializing_if = "Option::is_none")]
    pub u_acw_pod: Value,
    #[serde(rename = "U_ACW_COO", skip_serializing_if = "Option::is_none")]
    pub u_acw_coo: Value,
    #[serde(rename = "U_ACW_DeliveryFrom", skip_serializing_if = "Option::is_none")]
    pub u_acw_delivery_from: String,
    #[serde(rename = "U_ACW_DeliveryEnd", skip_serializing_if = "Option::is_none")]
    pub u_acw_delivery_end: String,
    #[serde(rename = "U_ACW_SalesTerms", skip_serializing_if = "Option::is_none")]
    pub u_acw_sales_terms: Value,
    #[serde(rename = "U_ACW_HSCode01", skip_serializing_if = "Option::is_none")]
    pub u_acw_hscode01: Value,
    #[serde(rename = "U_ACW_HSCode02", skip_serializing_if = "Option::is_none")]
    pub u_acw_hscode02: Value,
    #[serde(rename = "U_ACW_HSCode03", skip_serializing_if = "Option::is_none")]
    pub u_acw_hscode03: Value,
    #[serde(rename = "U_ACW_HSCode04", skip_serializing_if = "Option::is_none")]
    pub u_acw_hscode04: Value,
    #[serde(rename = "U_ACW_HSCode05", skip_serializing_if = "Option::is_none")]
    pub u_acw_hscode05: Value,
    #[serde(rename = "U_ACW_InBoxContent", skip_serializing_if = "Option::is_none")]
    pub u_acw_in_box_content: Value,
    #[serde(rename = "U_ACW_MasCarConten", skip_serializing_if = "Option::is_none")]
    pub u_acw_mas_car_conten: Value,
    #[serde(rename = "U_ACW_Certificate", skip_serializing_if = "Option::is_none")]
    pub u_acw_certificate: Value,
    #[serde(rename = "U_ACW_QCPass", skip_serializing_if = "Option::is_none")]
    pub u_acw_qcpass: Value,
    #[serde(rename = "U_TBD_Container_No", skip_serializing_if = "Option::is_none")]
    pub u_tbd_container_no: Value,
    #[serde(rename = "U_TBD_SO_Ref", skip_serializing_if = "Option::is_none")]
    pub u_tbd_so_ref: String,
    #[serde(rename = "U_TBD_Cust_Name", skip_serializing_if = "Option::is_none")]
    pub u_tbd_cust_name: String,
    #[serde(rename = "U_TBD_Cust_Cat", skip_serializing_if = "Option::is_none")]
    pub u_tbd_cust_cat: Value,
    #[serde(rename = "U_TBD_SO_No", skip_serializing_if = "Option::is_none")]
    pub u_tbd_so_no: String,
    #[serde(rename = "U_TBD_Assortment_1", skip_serializing_if = "Option::is_none")]
    pub u_tbd_assortment_1: Value,
    #[serde(rename = "U_TBD_Assortment_2", skip_serializing_if = "Option::is_none")]
    pub u_tbd_assortment_2: Value,
    #[serde(rename = "U_TBD_Assortment_3", skip_serializing_if = "Option::is_none")]
    pub u_tbd_assortment_3: Value,
    #[serde(rename = "U_TBD_Assortment_4", skip_serializing_if = "Option::is_none")]
    pub u_tbd_assortment_4: Value,
    #[serde(rename = "U_TBD_Assortment_5", skip_serializing_if = "Option::is_none")]
    pub u_tbd_assortment_5: Value,
    #[serde(rename = "U_TBD_Cust_No", skip_serializing_if = "Option::is_none")]
    pub u_tbd_cust_no: String,
    #[serde(rename = "U_TBD_Cust_Dept_No", skip_serializing_if = "Option::is_none")]
    pub u_tbd_cust_dept_no: Value,
    #[serde(rename = "U_TBD_GW", skip_serializing_if = "Option::is_none")]
    pub u_tbd_gw: Value,
    #[serde(rename = "U_TBD_NW", skip_serializing_if = "Option::is_none")]
    pub u_tbd_nw: Value,
    #[serde(rename = "U_TBD_GW_kg", skip_serializing_if = "Option::is_none")]
    pub u_tbd_gw_kg: Value,
    #[serde(rename = "U_TBD_NW_kg", skip_serializing_if = "Option::is_none")]
    pub u_tbd_nw_kg: Value,
    #[serde(rename = "U_TBD_Track_No", skip_serializing_if = "Option::is_none")]
    pub u_tbd_track_no: Value,
    #[serde(rename = "U_TBD_FH_Royalty", skip_serializing_if = "Option::is_none")]
    pub u_tbd_fh_royalty: f64,
    #[serde(rename = "U_ProjectID", skip_serializing_if = "Option::is_none")]
    pub u_project_id: Value,
    #[serde(rename = "U_State", skip_serializing_if = "Option::is_none")]
    pub u_state: Value,
    #[serde(rename = "U_Country", skip_serializing_if = "Option::is_none")]
    pub u_country: Value,
    #[serde(rename = "U_Productline", skip_serializing_if = "Option::is_none")]
    pub u_productline: String,
    #[serde(rename = "U_Division", skip_serializing_if = "Option::is_none")]
    pub u_division: String,
    #[serde(rename = "U_TranPrice", skip_serializing_if = "Option::is_none")]
    pub u_tran_price: f64,
    #[serde(rename = "U_LineDesc", skip_serializing_if = "Option::is_none")]
    pub u_line_desc: Value,
    #[serde(rename = "U_TBD_L_CM", skip_serializing_if = "Option::is_none")]
    pub u_tbd_l_cm: Value,
    #[serde(rename = "U_TBD_W_CM", skip_serializing_if = "Option::is_none")]
    pub u_tbd_w_cm: Value,
    #[serde(rename = "U_TBD_H_CM", skip_serializing_if = "Option::is_none")]
    pub u_tbd_h_cm: Value,
    #[serde(rename = "U_TBD_Cust_Des", skip_serializing_if = "Option::is_none")]
    pub u_tbd_cust_des: Value,
    #[serde(rename = "U_TBD_FSD", skip_serializing_if = "Option::is_none")]
    pub u_tbd_fsd: Value,
    #[serde(rename = "U_TBD_CID", skip_serializing_if = "Option::is_none")]
    pub u_tbd_cid: Value,
    #[serde(rename = "U_TBD_WM_Week", skip_serializing_if = "Option::is_none")]
    pub u_tbd_wm_week: Value,
    #[serde(rename = "U_TBD_Shipping_Desc", skip_serializing_if = "Option::is_none")]
    pub u_tbd_shipping_desc: Value,
    #[serde(rename = "U_ItemCategory", skip_serializing_if = "Option::is_none")]
    pub u_item_category: Value,
    #[serde(rename = "U_TBD_CM", skip_serializing_if = "Option::is_none")]
    pub u_tbd_cm: Value,
    #[serde(rename = "U_TBD_Royalty", skip_serializing_if = "Option::is_none")]
    pub u_tbd_royalty: String,
    #[serde(rename = "U_Origin_Whs", skip_serializing_if = "Option::is_none")]
    pub u_origin_whs: Value,
    #[serde(rename = "U_ECSB1_BREX_ID", skip_serializing_if = "Option::is_none")]
    pub u_ecsb1_brex_id: Value,
    #[serde(rename = "U_TBD_Manu_Address", skip_serializing_if = "Option::is_none")]
    pub u_tbd_manu_address: Value,
    #[serde(rename = "U_WSA_ID", skip_serializing_if = "Option::is_none")]
    pub u_wsa_id: Value,
    #[serde(rename = "U_WhsName", skip_serializing_if = "Option::is_none")]
    pub u_whs_name: Value,
    #[serde(rename = "U_PACK_SLIP_NBR", skip_serializing_if = "Option::is_none")]
    pub u_pack_slip_nbr: Value,
    #[serde(rename = "U_ARN_NUMBER", skip_serializing_if = "Option::is_none")]
    pub u_arn_number: Value,
    #[serde(rename = "U_WSA_FileName", skip_serializing_if = "Option::is_none")]
    pub u_wsa_file_name: Value,
    #[serde(rename = "U_Transaction_Date", skip_serializing_if = "Option::is_none")]
    pub u_transaction_date: Value,
    #[serde(rename = "U_TransactionUID", skip_serializing_if = "Option::is_none")]
    pub u_transaction_uid: Value,
    #[serde(rename = "U_ShipmentUID", skip_serializing_if = "Option::is_none")]
    pub u_shipment_uid: Value,
    #[serde(rename = "U_Carrier_Code", skip_serializing_if = "Option::is_none")]
    pub u_carrier_code: Value,
    #[serde(rename = "U_CarrierID_Number", skip_serializing_if = "Option::is_none")]
    pub u_carrier_id_number: Value,
    #[serde(rename = "U_ContainerID", skip_serializing_if = "Option::is_none")]
    pub u_container_id: Value,
    #[serde(rename = "U_PackingListID", skip_serializing_if = "Option::is_none")]
    pub u_packing_list_id: Value,
    #[serde(rename = "U_BOL_NBR", skip_serializing_if = "Option::is_none")]
    pub u_bol_nbr: Value,
    #[serde(rename = "U_Distributor_Price", skip_serializing_if = "Option::is_none")]
    pub u_distributor_price: f64,
    #[serde(rename = "U_TBD_Shipment_Priority", skip_serializing_if = "Option::is_none")]
    pub u_tbd_shipment_priority: String,
    #[serde(rename = "U_PRO_NBR", skip_serializing_if = "Option::is_none")]
    pub u_pro_nbr: Value,
    #[serde(rename = "U_ECSB1_PLength", skip_serializing_if = "Option::is_none")]
    pub u_ecsb1_plength: Value,
    #[serde(rename = "U_ECSB1_PWidth", skip_serializing_if = "Option::is_none")]
    pub u_ecsb1_pwidth: Value,
    #[serde(rename = "U_ECSB1_PHeight", skip_serializing_if = "Option::is_none")]
    pub u_ecsb1_pheight: Value,
    #[serde(rename = "U_ECSB1_PVolume", skip_serializing_if = "Option::is_none")]
    pub u_ecsb1_pvolume: Value,
    #[serde(rename = "U_ECSB1_PWeight", skip_serializing_if = "Option::is_none")]
    pub u_ecsb1_pweight: Value,
    #[serde(rename = "LineTaxJurisdictions", skip_serializing_if = "Option::is_none")]
    pub line_tax_jurisdictions: Vec<Value>,
    #[serde(rename = "DocumentLineAdditionalExpenses", skip_serializing_if = "Option::is_none")]
    pub document_line_additional_expenses: Vec<Value>,
    #[serde(rename = "WithholdingTaxLines", skip_serializing_if = "Option::is_none")]
    pub withholding_tax_lines: Vec<Value>,
    #[serde(rename = "SerialNumbers", skip_serializing_if = "Option::is_none")]
    pub serial_numbers: Vec<Value>,
    #[serde(rename = "BatchNumbers", skip_serializing_if = "Option::is_none")]
    pub batch_numbers: Vec<Value>,
    #[serde(rename = "DocumentLinesBinAllocations", skip_serializing_if = "Option::is_none")]
    pub document_lines_bin_allocations: Vec<Value>,
}
*/
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaxExtension {
    #[serde(rename = "TaxId0")]
    pub tax_id0: Value,
    #[serde(rename = "TaxId1")]
    pub tax_id1: Value,
    #[serde(rename = "TaxId2")]
    pub tax_id2: Value,
    #[serde(rename = "TaxId3")]
    pub tax_id3: Value,
    #[serde(rename = "TaxId4")]
    pub tax_id4: Value,
    #[serde(rename = "TaxId5")]
    pub tax_id5: Value,
    #[serde(rename = "TaxId6")]
    pub tax_id6: Value,
    #[serde(rename = "TaxId7")]
    pub tax_id7: Value,
    #[serde(rename = "TaxId8")]
    pub tax_id8: Value,
    #[serde(rename = "TaxId9")]
    pub tax_id9: Value,
    #[serde(rename = "State")]
    pub state: Value,
    #[serde(rename = "County")]
    pub county: Value,
    #[serde(rename = "Incoterms")]
    pub incoterms: Value,
    #[serde(rename = "Vehicle")]
    pub vehicle: Value,
    #[serde(rename = "VehicleState")]
    pub vehicle_state: Value,
    #[serde(rename = "NFRef")]
    pub nfref: Value,
    #[serde(rename = "Carrier")]
    pub carrier: Value,
    #[serde(rename = "PackQuantity")]
    pub pack_quantity: Value,
    #[serde(rename = "PackDescription")]
    pub pack_description: Value,
    #[serde(rename = "Brand")]
    pub brand: Value,
    #[serde(rename = "ShipUnitNo")]
    pub ship_unit_no: Value,
    #[serde(rename = "NetWeight")]
    pub net_weight: f64,
    #[serde(rename = "GrossWeight")]
    pub gross_weight: f64,
    #[serde(rename = "StreetS")]
    pub street_s: String,
    #[serde(rename = "BlockS")]
    pub block_s: String,
    #[serde(rename = "BuildingS")]
    pub building_s: String,
    #[serde(rename = "CityS")]
    pub city_s: String,
    #[serde(rename = "ZipCodeS")]
    pub zip_code_s: String,
    #[serde(rename = "CountyS")]
    pub county_s: Value,
    #[serde(rename = "StateS")]
    pub state_s: String,
    #[serde(rename = "CountryS")]
    pub country_s: String,
    #[serde(rename = "StreetB")]
    pub street_b: String,
    #[serde(rename = "BlockB")]
    pub block_b: String,
    #[serde(rename = "BuildingB")]
    pub building_b: Value,
    #[serde(rename = "CityB")]
    pub city_b: String,
    #[serde(rename = "ZipCodeB")]
    pub zip_code_b: String,
    #[serde(rename = "CountyB")]
    pub county_b: Value,
    #[serde(rename = "StateB")]
    pub state_b: String,
    #[serde(rename = "CountryB")]
    pub country_b: String,
    #[serde(rename = "ImportOrExport")]
    pub import_or_export: Value,
    #[serde(rename = "MainUsage")]
    pub main_usage: Value,
    #[serde(rename = "GlobalLocationNumberS")]
    pub global_location_number_s: String,
    #[serde(rename = "GlobalLocationNumberB")]
    pub global_location_number_b: Value,
    #[serde(rename = "TaxId12")]
    pub tax_id12: Value,
    #[serde(rename = "TaxId13")]
    pub tax_id13: Value,
    #[serde(rename = "BillOfEntryNo")]
    pub bill_of_entry_no: Value,
    #[serde(rename = "BillOfEntryDate")]
    pub bill_of_entry_date: Value,
    #[serde(rename = "OriginalBillOfEntryNo")]
    pub original_bill_of_entry_no: Value,
    #[serde(rename = "OriginalBillOfEntryDate")]
    pub original_bill_of_entry_date: Value,
    #[serde(rename = "ImportOrExportType")]
    pub import_or_export_type: String,
    #[serde(rename = "PortCode")]
    pub port_code: Value,
    #[serde(rename = "DocEntry")]
    pub doc_entry: i64,
    #[serde(rename = "BoEValue")]
    pub bo_evalue: f64,
    #[serde(rename = "ClaimRefund")]
    pub claim_refund: Value,
    #[serde(rename = "DifferentialOfTaxRate")]
    pub differential_of_tax_rate: Value,
    #[serde(rename = "IsIGSTAccount")]
    pub is_igstaccount: Value,
    #[serde(rename = "TaxId14")]
    pub tax_id14: Value,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
    #[serde(rename = "ShipToZipCode", skip_serializing_if = "Option::is_none")]
    pub ship_to_zip_code: Option<String>,
    #[serde(rename = "ShipToCounty", skip_serializing_if = "Option::is_none")]
    pub ship_to_county: Option<Value>,
    #[serde(rename = "ShipToState", skip_serializing_if = "Option::is_none")]
    pub ship_to_state: Option<String>,
    #[serde(rename = "ShipToCountry", skip_serializing_if = "Option::is_none")]
    pub ship_to_country: Option<String>,
    #[serde(rename = "ShipToAddressType", skip_serializing_if = "Option::is_none")]
    pub ship_to_address_type: Option<String>,
    #[serde(rename = "BillToStreet", skip_serializing_if = "Option::is_none")]
    pub bill_to_street: Option<String>,
    #[serde(rename = "BillToStreetNo", skip_serializing_if = "Option::is_none")]
    pub bill_to_street_no: Option<String>,
    #[serde(rename = "BillToBlock", skip_serializing_if = "Option::is_none")]
    pub bill_to_block: Option<String>,
    #[serde(rename = "BillToBuilding", skip_serializing_if = "Option::is_none")]
    pub bill_to_building: Option<Value>,
    #[serde(rename = "BillToCity", skip_serializing_if = "Option::is_none")]
    pub bill_to_city: Option<String>,
    #[serde(rename = "BillToZipCode", skip_serializing_if = "Option::is_none")]
    pub bill_to_zip_code: Option<String>,
    #[serde(rename = "BillToCounty", skip_serializing_if = "Option::is_none")]
    pub bill_to_county: Option<Value>,
    #[serde(rename = "BillToState", skip_serializing_if = "Option::is_none")]
    pub bill_to_state: Option<String>,
    #[serde(rename = "BillToCountry", skip_serializing_if = "Option::is_none")]
    pub bill_to_country: Option<String>,
    #[serde(rename = "BillToAddressType", skip_serializing_if = "Option::is_none")]
    pub bill_to_address_type: Option<Value>,
    #[serde(
        rename = "ShipToGlobalLocationNumber",
        skip_serializing_if = "Option::is_none"
    )]
    pub ship_to_global_location_number: Option<String>,
    #[serde(
        rename = "BillToGlobalLocationNumber",
        skip_serializing_if = "Option::is_none"
    )]
    pub bill_to_global_location_number: Option<Value>,
    #[serde(rename = "ShipToAddress2", skip_serializing_if = "Option::is_none")]
    pub ship_to_address2: Option<String>,
    #[serde(rename = "ShipToAddress3", skip_serializing_if = "Option::is_none")]
    pub ship_to_address3: Option<String>,
    #[serde(rename = "BillToAddress2", skip_serializing_if = "Option::is_none")]
    pub bill_to_address2: Option<String>,
    #[serde(rename = "BillToAddress3", skip_serializing_if = "Option::is_none")]
    pub bill_to_address3: Option<Value>,
    #[serde(rename = "PlaceOfSupply", skip_serializing_if = "Option::is_none")]
    pub place_of_supply: Option<Value>,
    #[serde(
        rename = "PurchasePlaceOfSupply",
        skip_serializing_if = "Option::is_none"
    )]
    pub purchase_place_of_supply: Option<Value>,
    #[serde(rename = "DocEntry", skip_serializing_if = "Option::is_none")]
    pub doc_entry: Option<i64>,
    #[serde(rename = "GoodsIssuePlaceBP", skip_serializing_if = "Option::is_none")]
    pub goods_issue_place_bp: Option<Value>,
    #[serde(
        rename = "GoodsIssuePlaceCNPJ",
        skip_serializing_if = "Option::is_none"
    )]
    pub goods_issue_place_cnpj: Option<Value>,
    #[serde(rename = "GoodsIssuePlaceCPF", skip_serializing_if = "Option::is_none")]
    pub goods_issue_place_cpf: Option<Value>,
    #[serde(
        rename = "GoodsIssuePlaceStreet",
        skip_serializing_if = "Option::is_none"
    )]
    pub goods_issue_place_street: Option<Value>,
    #[serde(
        rename = "GoodsIssuePlaceStreetNo",
        skip_serializing_if = "Option::is_none"
    )]
    pub goods_issue_place_street_no: Option<Value>,
    #[serde(
        rename = "GoodsIssuePlaceBuilding",
        skip_serializing_if = "Option::is_none"
    )]
    pub goods_issue_place_building: Option<Value>,
    #[serde(rename = "GoodsIssuePlaceZip", skip_serializing_if = "Option::is_none")]
    pub goods_issue_place_zip: Option<Value>,
    #[serde(
        rename = "GoodsIssuePlaceBlock",
        skip_serializing_if = "Option::is_none"
    )]
    pub goods_issue_place_block: Option<Value>,
    #[serde(
        rename = "GoodsIssuePlaceCity",
        skip_serializing_if = "Option::is_none"
    )]
    pub goods_issue_place_city: Option<Value>,
    #[serde(
        rename = "GoodsIssuePlaceCounty",
        skip_serializing_if = "Option::is_none"
    )]
    pub goods_issue_place_county: Option<Value>,
    #[serde(
        rename = "GoodsIssuePlaceState",
        skip_serializing_if = "Option::is_none"
    )]
    pub goods_issue_place_state: Option<Value>,
    #[serde(
        rename = "GoodsIssuePlaceCountry",
        skip_serializing_if = "Option::is_none"
    )]
    pub goods_issue_place_country: Option<Value>,
    #[serde(
        rename = "GoodsIssuePlacePhone",
        skip_serializing_if = "Option::is_none"
    )]
    pub goods_issue_place_phone: Option<Value>,
    #[serde(
        rename = "GoodsIssuePlaceEMail",
        skip_serializing_if = "Option::is_none"
    )]
    pub goods_issue_place_email: Option<Value>,
    #[serde(
        rename = "GoodsIssuePlaceDepartureDate",
        skip_serializing_if = "Option::is_none"
    )]
    pub goods_issue_place_departure_date: Option<Value>,
    #[serde(rename = "DeliveryPlaceBP", skip_serializing_if = "Option::is_none")]
    pub delivery_place_bp: Option<Value>,
    #[serde(rename = "DeliveryPlaceCNPJ", skip_serializing_if = "Option::is_none")]
    pub delivery_place_cnpj: Option<Value>,
    #[serde(rename = "DeliveryPlaceCPF", skip_serializing_if = "Option::is_none")]
    pub delivery_place_cpf: Option<Value>,
    #[serde(
        rename = "DeliveryPlaceStreet",
        skip_serializing_if = "Option::is_none"
    )]
    pub delivery_place_street: Option<Value>,
    #[serde(
        rename = "DeliveryPlaceStreetNo",
        skip_serializing_if = "Option::is_none"
    )]
    pub delivery_place_street_no: Option<Value>,
    #[serde(
        rename = "DeliveryPlaceBuilding",
        skip_serializing_if = "Option::is_none"
    )]
    pub delivery_place_building: Option<Value>,
    #[serde(rename = "DeliveryPlaceZip", skip_serializing_if = "Option::is_none")]
    pub delivery_place_zip: Option<Value>,
    #[serde(rename = "DeliveryPlaceBlock", skip_serializing_if = "Option::is_none")]
    pub delivery_place_block: Option<Value>,
    #[serde(rename = "DeliveryPlaceCity", skip_serializing_if = "Option::is_none")]
    pub delivery_place_city: Option<Value>,
    #[serde(
        rename = "DeliveryPlaceCounty",
        skip_serializing_if = "Option::is_none"
    )]
    pub delivery_place_county: Option<Value>,
    #[serde(rename = "DeliveryPlaceState", skip_serializing_if = "Option::is_none")]
    pub delivery_place_state: Option<Value>,
    #[serde(
        rename = "DeliveryPlaceCountry",
        skip_serializing_if = "Option::is_none"
    )]
    pub delivery_place_country: Option<Value>,
    #[serde(rename = "DeliveryPlacePhone", skip_serializing_if = "Option::is_none")]
    pub delivery_place_phone: Option<Value>,
    #[serde(rename = "DeliveryPlaceEMail", skip_serializing_if = "Option::is_none")]
    pub delivery_place_email: Option<Value>,
    #[serde(
        rename = "DeliveryPlaceDepartureDate",
        skip_serializing_if = "Option::is_none"
    )]
    pub delivery_place_departure_date: Option<Value>,
    #[serde(rename = "U_VATLOCIDS", skip_serializing_if = "Option::is_none")]
    pub u_vatlocids: Option<String>,
    #[serde(rename = "U_VATLOCIDB", skip_serializing_if = "Option::is_none")]
    pub u_vatlocidb: Option<Value>,
    #[serde(rename = "U_SecondIDS", skip_serializing_if = "Option::is_none")]
    pub u_second_ids: Option<String>,
    #[serde(rename = "U_SecondIDB", skip_serializing_if = "Option::is_none")]
    pub u_second_idb: Option<Value>,
    #[serde(rename = "U_ThirdIDS", skip_serializing_if = "Option::is_none")]
    pub u_third_ids: Option<String>,
    #[serde(rename = "U_ThirdIDB", skip_serializing_if = "Option::is_none")]
    pub u_third_idb: Option<Value>,
    #[serde(rename = "U_ConsigneeS", skip_serializing_if = "Option::is_none")]
    pub u_consignee_s: Option<String>,
    #[serde(rename = "U_ConsigneeB", skip_serializing_if = "Option::is_none")]
    pub u_consignee_b: Option<Value>,
        #[serde(rename = "U_ContactNameS", skip_serializing_if = "Option::is_none")]
    pub u_contact_name_s: Option<String>,
        #[serde(rename = "U_ContactNameB", skip_serializing_if = "Option::is_none")]
    pub u_contact_name_b: Option<String>,
        #[serde(rename = "U_ContactPhoneS", skip_serializing_if = "Option::is_none")]
    pub u_contact_phone_s: Option<String>,
        #[serde(rename = "U_ContactPhoneB", skip_serializing_if = "Option::is_none")]
    pub u_contact_phone_b: Option<String>,
        #[serde(rename = "U_ContactEmailS", skip_serializing_if = "Option::is_none")]
    pub u_contact_email_s: Option<String>,
        #[serde(rename = "U_ContactEmailB", skip_serializing_if = "Option::is_none")]
    pub u_contact_email_b: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentReference {
    #[serde(rename = "DocEntry")]
    pub doc_entry: i64,
    #[serde(rename = "LineNumber")]
    pub line_number: i64,
    #[serde(rename = "RefDocEntr")]
    pub ref_doc_entr: i64,
    #[serde(rename = "RefDocNum")]
    pub ref_doc_num: i64,
    #[serde(rename = "ExtDocNum")]
    pub ext_doc_num: Value,
    #[serde(rename = "RefObjType")]
    pub ref_obj_type: String,
    #[serde(rename = "AccessKey")]
    pub access_key: Value,
    #[serde(rename = "IssueDate")]
    pub issue_date: String,
    #[serde(rename = "IssuerCNPJ")]
    pub issuer_cnpj: Value,
    #[serde(rename = "IssuerCode")]
    pub issuer_code: Value,
    #[serde(rename = "Model")]
    pub model: Value,
    #[serde(rename = "Series")]
    pub series: Value,
    #[serde(rename = "Number")]
    pub number: Value,
    #[serde(rename = "RefAccKey")]
    pub ref_acc_key: Value,
    #[serde(rename = "RefAmount")]
    pub ref_amount: f64,
    #[serde(rename = "SubSeries")]
    pub sub_series: Value,
    #[serde(rename = "Remark")]
    pub remark: Value,
    #[serde(rename = "LinkRefTyp")]
    pub link_ref_typ: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Token {
    #[serde(rename = "odata.metadata", skip_serializing_if = "Option::is_none")]
    pub odata_metadata: Option<String>,
    #[serde(rename = "SessionId")]
    pub session_id: String,
    #[serde(rename = "Version", skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(rename = "SessionTimeout", skip_serializing_if = "Option::is_none")]
    pub session_timeout: Option<i64>,
}
