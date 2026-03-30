from fastapi import FastAPI, HTTPException, Depends, Header
from pydantic import BaseModel
from typing import Optional, Dict

app = FastAPI()

# -----------------------------
# Fake Database
# -----------------------------
orders_db: Dict[int, dict] = {}

# -----------------------------
# Auth (Simple Token)
# -----------------------------
API_TOKEN = "mysecrettoken"

def authenticate(authorization: Optional[str] = Header(None)):
    if authorization != f"Bearer {API_TOKEN}":
        raise HTTPException(status_code=401, detail="Unauthorized")
    return True

# -----------------------------
# Models
# -----------------------------
class Order(BaseModel):
    id: int
    item: str
    quantity: int
    price: float

# -----------------------------
# POST - Create Order
# -----------------------------
@app.post("/orders")
def create_order(order: Order, auth: bool = Depends(authenticate)):
    if order.id in orders_db:
        raise HTTPException(status_code=400, detail="Order already exists")

    orders_db[order.id] = order.dict()

    return {
        "data": {
            "message": "Order created",
            "order": orders_db[order.id]
        }
    }

# -----------------------------
# GET - Get Order
# -----------------------------
@app.get("/orders/{order_id}")
def get_order(order_id: int, auth: bool = Depends(authenticate)):
    if order_id not in orders_db:
        raise HTTPException(status_code=404, detail="Order not found")

    return {
        "data": {
            "order": orders_db[order_id]
        }
    }

# -----------------------------
# PUT - Update Order
# -----------------------------
@app.put("/orders/{order_id}")
def update_order(order_id: int, order: Order, auth: bool = Depends(authenticate)):
    if order_id not in orders_db:
        raise HTTPException(status_code=404, detail="Order not found")

    orders_db[order_id] = order.dict()

    return {
        "data": {
            "message": "Order updated",
            "order": orders_db[order_id]
        }
    }