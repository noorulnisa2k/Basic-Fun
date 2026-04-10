from fastapi import FastAPI
from pydantic import BaseModel

app = FastAPI()

class ShipToRequest(BaseModel):
    card_type: str
    gln: str
    dummy: str
    card_code: str

@app.post("/sap-test")
def sap_test(data: ShipToRequest):

    # simulate SAP response
    return {
        "value": [
            {
                "BusinessPartners/BPAddresses": {
                    "AddressName": "TEST_SHIP_TO_ADDRESS"
                }
            }
        ]
    }