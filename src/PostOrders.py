import requests
import os
import shutil
import sys
import json
import logging
import time
import smtplib
import ssl
from email.message import EmailMessage
from datetime import datetime
from dotenv import load_dotenv

# creds file read at start
# ~po at the start
# url, https:
# log and error file name
# save error in a file
# ~multiple orders in one file
# ~retry for 502
# ~start with BPID in logs and create one logs file everyday
load_dotenv()
base_dir = ''
# creds = ''
input_dir = base_dir + 'Input'
output_dir = base_dir + 'Output'
error_dir = base_dir + 'Errors'
logs_dir = base_dir + 'Logs'
process_id = '12345678'
session_id = ''
login_at = ''
session_expires_in = ''
cookies = ''
header = {"Content-Type":"application/json","Connection": "keep-alive","AuthKey":"MTB_AEC_1ofd1n19zk1O9UGGQPBMY8"}
collect_errors = ''
sender_email= os.getenv("SENDER_EMAIL")
sender_pass = os.getenv("SENDER_PASS")
receiver_email = os.getenv("RECEIVER_EMAIL")
server_port = os.getenv("SERVER_PORT")
server_host = os.getenv("SERVER_HOST")

def req(method, url, **kwargs):
  for i in range(3):
    try:
      if method == 'post':
        response = requests.post(url, **kwargs, timeout=300)
      elif method == 'get':
        response = requests.get(url, **kwargs, timeout=300)
      
      if response.status_code in {400, 401, 500, 501, 502, 503, 504}:
          try:
              resp_json = response.json()
              error_code = resp_json.get('error', {}).get('code')
              error_msg = resp_json.get('error', {}).get('message', {}).get('value')
          except ValueError:
              # Catch JSONDecodeError if the response is HTML (like 502 bad gateway)
              error_code = None
              error_msg = response.text

          if error_code != -1116:
              logging.warning(f"Status code is {response.status_code}, Response: {error_msg}")
              if i > 1:
                logging.error(f"Request failed 3 times: {error_msg}")
                if response.status_code in {401, 500, 501, 502, 503, 504}:
                  raise Exception({
                    "status" : response.status_code,
                    "details" : error_msg
                  })
                raise ValueError(error_msg)
              
              # Automatically re-login if the cookie expired unexpectedly!
              if response.status_code == 401:
                  login()
                  # Update kwargs so the next retry uses the new session cookie
                  if 'cookies' in kwargs:
                      kwargs['cookies'] = globals()['cookies']

              time.sleep(30)
          else:
            return response

      else:
        return response

    except requests.exceptions.ConnectionError as e:
      logging.error(e)
      if i > 0:
        raise Exception(e)
      time.sleep(30)

    except requests.exceptions.RequestException as e:
      logging.error(e)
      raise Exception(e)
    
    except Exception as e:
      logging.error("Something went wrong in request")
      logging.error(e)
      raise Exception(e)


def AFFES_item_backorder(id, item, warehouse, qty):
    if is_session_expired():
        login()
    cookies = {
            "B1SESSION": id
        }
    header = {"Content-Type":"application/json","Connection": "keep-alive","AuthKey":"MTB_AEC_1ofd1n19zk1O9UGGQPBMY8"}
    url = f"https://f08sl.softengineapps.com:50000/b1s/v1/Items('{item}')"
    response = req('get', url, headers=header, cookies=cookies)
    if response.status_code == 200:
      for i in response.json()['ItemWarehouseInfoCollection']:
        print(i['WarehouseCode'], warehouse)
        if i['WarehouseCode'] == warehouse:
          avail_qty = int(i['InStock']) - int(i['Committed'])
          if int(avail_qty)>= qty:
            return True
          else:
            return None


def is_session_expired():

  # Get the current date and time
  current_time = datetime.now()

  format = "%Y-%m-%d %H:%M:%S"
  format_fetched_time = datetime.strptime(login_at, format)

  # Calculate the difference in seconds
  seconds = (current_time - format_fetched_time).total_seconds()

  return True if seconds >= (session_expires_in - 2) else False
    
def remove_replace_me(data):
    # Function to recursively remove "ReplaceMe" keys
    def remove_replace_me_helper(data):
        if isinstance(data, dict):
            data = {key: remove_replace_me_helper(value) for key, value in data.items() if key != "ReplaceMe"}
            data = {key: value for key, value in data.items() if value or isinstance(value, bool)}  # Remove empty dictionaries and False values
            return data
        elif isinstance(data, list):
            return [remove_replace_me_helper(item) for item in data]
        else:
            return data

    # Remove "ReplaceMe" keys
    cleaned_data = remove_replace_me_helper(data)
    return cleaned_data
    
def remove_empty_dicts(data):
    if isinstance(data, dict):
        return {key: remove_empty_dicts(value) for key, value in data.items() if value or isinstance(value, bool)}
    elif isinstance(data, list):
        return [remove_empty_dicts(item) for item in data if item]
    else:
        return data

def login():
  global session_id, session_expires_in, cookies
  data ={}
  env_file_path = "/edi_data/MichaelTodd/scripts/SAPB1/src/cred.env"
  if os.path.exists(env_file_path):
      with open(env_file_path, "r") as file:
          lines = file.readlines()
          for line in lines:
              # Split the line into key and value
              key, value = line.strip().split("=")
              data[key] = value
  
  url = "https://f08sl.softengineapps.com:50000/b1s/v1/Login"
  # response = requests.post(url, headers=header, json=data)
  response = req('post', url, headers=header, json=data)
  if response.status_code == 200:
    session_id = response.cookies.get("B1SESSION")
    cookies = {"B1SESSION": session_id}
    session_expires_in = response.json().get("SessionTimeout") * 60
    # return session_id
  else:
    logging.error(f"Invalid credentials: {response.json()['error']['message']['value']}")
    raise Exception(f"Invalid credentials")

def get_ST(id, data):
    if is_session_expired():
      login()
    result = data.split(',')
    url = f"https://f08sl.softengineapps.com:50000/b1s/v1/$crossjoin(BusinessPartners,BusinessPartners/BPAddresses)?$expand=BusinessPartners($select=CardType,CardCode),BusinessPartners/BPAddresses($select=AddressName,AddressType,GlobalLocationNumber)&$filter=BusinessPartners/CardCode eq BusinessPartners/BPAddresses/BPCode and BusinessPartners/CardCode eq '{result[3]}' and BusinessPartners/BPAddresses/GlobalLocationNumber eq '{result[1]}' and BusinessPartners/CardType eq '{result[0]}' and BusinessPartners/BPAddresses/AddressType eq 'bo_ShipTo' &$top=1"
    # response = requests.get(url, headers=header, cookies=cookies)
    response = req('get', url, headers=header, cookies=cookies)
    if len(response.json()['value'])>0:
      return response.json()['value'][0]['BusinessPartners/BPAddresses']['AddressName']
    else:
      return data

def incomming_payment(id,downpayment, order):
  income_payment = {}
  income_payment["DocType"] =  "rCustomer"
  income_payment["DocDate"] = downpayment["DocDate"]
  income_payment["CardCode"] = downpayment["CardCode"]
  income_payment["Address"] = downpayment["Address"]
  income_payment["TransferAccount"] = order['U_PaymentMethod']
  income_payment["TransferSum"] = downpayment["DocTotal"]
  if len(downpayment["NumAtCard"]) <= 8:
    income_payment["CounterReference"] = downpayment["NumAtCard"]
  income_payment["TransferReference"] = downpayment["NumAtCard"]
  income_payment["DueDate"] = downpayment["DocumentInstallments"][0]["DueDate"]
  income_payment["BPLID"] = downpayment["BPL_IDAssignedToInvoice"]
  if downpayment['CardCode'] == 'C2068':
    rule = ''
    if downpayment["BPLName"] ==   "Spa Sciences":
      rule = "SS"
    else:
       rule = "MTB"
    income_payment["PaymentInvoices"] = [{"LineNum": 0, "DocEntry": downpayment["DocEntry"],"SumApplied": downpayment["DocTotal"],"InvoiceType": "it_DownPayment","DistributionRule": rule,"DistributionRule2": "ECOMM","DistributionRule3": "C2068",}]
  else:
      income_payment["PaymentInvoices"] = [{"LineNum": 0, "DocEntry": downpayment["DocEntry"],"SumApplied": downpayment["DocTotal"],"InvoiceType": "it_DownPayment"}]

  if is_session_expired():
    login()
  url = "https://f08sl.softengineapps.com:50000/b1s/v1/IncomingPayments"
  # incomming = requests.post(url, headers=header, cookies=cookies, json=income_payment)

  incomming = req('post', url, headers=header, cookies=cookies, json=income_payment)
  if incomming.status_code==201:
    logging.info(f"Incomming Payment generated for following PO# {po_number}")
    pass
  else:
    logging.error(f"Failed to generate Incomming payment for following PO# {po_number}, Error: {incomming.json()['error']['message']['value']}")
    print(incomming.text)
    raise Exception(f"\n{incomming.status_code} Failed to generate Incomming payment \nResposne: {incomming.text}")

def down_payment(id,order):
  if is_session_expired():
    login()
  downpayment = {}
  url = "https://f08sl.softengineapps.com:50000/b1s/v1/DownPayments"
  downpayment["DocDate"] = order['DocDate']
  downpayment["DocDueDate"] = order['DocDueDate']
  downpayment["CardCode"] = order['CardCode']
  downpayment["Address"] = order['Address']
  downpayment["NumAtCard"] = order['NumAtCard']
  downpayment["Comments"] = order['Comments']
  downpayment["ShipToCode"] = order['ShipToCode']
  downpayment["Address2"] = order['Address2']
  downpayment["U_Channel"] = order['U_Channel']
  downpayment["DownPaymentType"] = "dptInvoice"
  downpayment["U_PaymentMethod"] = order['U_PaymentMethod']
  discount = sum(expense["LineTotal"] for expense in order["DocumentAdditionalExpenses"])
  if int(order['DocTotal']) > 0:
    if discount < 0 and discount:
      downpayment["DownPaymentPercentage"] = (100 + ((discount/(abs(discount)+float(order['DocTotal'])))*100))
    elif discount >0 and discount:
      downpayment["DownPaymentPercentage"] = (100 + ((abs(discount)/(float(order['DocTotal'])-abs(discount)))*100))
  else:
    downpayment["DownPaymentPercentage"] = 100
  
  downpayment["BPL_IDAssignedToInvoice"] = order['BPL_IDAssignedToInvoice']
  downpayment["DocumentLines"] = [{"BaseType":17,"BaseEntry":order['DocumentLines'][i]['DocEntry'],"BaseLine":order['DocumentLines'][i]['LineNum']} for i in range(len(order['DocumentLines']))]
  
  payment = req('post', url, headers=header, cookies=cookies, json=downpayment)
  if payment.status_code == 201:
    logging.info(f"Down payment generated for following PO# {po_number}")
    incomming_payment(id,payment.json(), order)
  else:
    logging.error(f"Failed to generate Down payment for following PO# {po_number}, Error: {payment.json()['error']['message']['value']}")
    print(payment.text)
    raise Exception(f"\n{payment.status_code} Failed to generate Down payment. \nResposne: {payment.text}")

def post_order(data,file_name,output_dir):

    international = False
    international_phone = ''
    international_email = ''
    international_courier = ''
    if is_session_expired():
      login()
    url = "https://f08sl.softengineapps.com:50000/b1s/v1/Orders"
    if "ShipToCode" in data:
      data['ShipToCode'] = get_ST(session_id,data['ShipToCode'])
    if data['CardCode'] == 'C1034':
       if 'U_Courier' in data:
          if data['U_Courier'] == 'Smart Parcel - Canada Post DDP (Battery)':
            international_courier = 2539
          elif data['U_Courier'] == 'Smart Parcel - AU Post (Battery)':
            international_courier = 2538
          else:
            international_courier = 0
          data['U_Courier'] = international_courier
    print('data to post')
    print(data)
    # order_posting = requests.post(url, headers=header, cookies=cookies, json=data)

    order_posting = req('post', url, headers=header, cookies=cookies, json=data)
    if order_posting.status_code == 201:
      name = order_posting.json()
      if "U_InternationalOrder" in name:
        if name['U_InternationalOrder'] != "" and name['U_InternationalOrder'] is not None:
          international = True
          if "U_OrderPhone" in name:
            international_phone =  name['U_OrderPhone']
          if "U_OrderEmail" in name:
            international_email = name['U_OrderEmail']
      if international:
        file_name = f"FLOSHIP_{name['BPLName']}-940_{name['CardCode']}_{name['DocNum']}_{po_number}.txt".replace(" ","")
      elif name['DocumentLines'][0]['WarehouseCode'] == 'AM - SS' or name['DocumentLines'][0]['WarehouseCode'] == 'AM - MTB':
        file_name = f"{name['BPLName']}-AM-940_{name['CardCode']}_{name['DocNum']}_{po_number}.txt".replace(" ","")
      elif name['DocumentLines'][0]['WarehouseCode'] == 'SBGA-SS':
        file_name = f"{name['BPLName']}-SBGA-SS-940_{name['CardCode']}_{name['DocNum']}_{po_number}.txt".replace(" ","")
      elif name['DocumentLines'][0]['WarehouseCode'] == 'SBGA-MT':
        file_name = f"{name['BPLName']}-SBGA-MT-940_{name['CardCode']}_{name['DocNum']}_{po_number}.txt".replace(" ","")

      if name['Confirmed'] != 'tNO':
        file_path1 = os.path.join(output_dir, file_name)
        
        if name['CardCode'] in ['C1034','C1035','C2068','C2059','C2061','C2070','C2072','C2071','C2060','C2074','C2058','C2075', 'C2077']:
          pass
        else:
          with open(file_path1, 'w') as file:
            file.write(order_posting.text)
            file.close()

        logging.info(f"940 generated for the following PO# {po_number}")
        if name['CardCode'] == 'C2025':
          # path2 = "/edi_data/MichaelTodd/sftp/SAPB1/out/855"
          path870 = "/edi_data/MichaelTodd/sftp/SAPB1/out/870"
          logging.info(f"855 generated for the following PO# {po_number}")
          file_name870 = f"{name['BPLName']}-{name['CardCode']}-870_{name['DocNum']}_{po_number}.txt".replace(" ","")
          file_path870 = os.path.join(path870, file_name870)
          with open(file_path870, 'w') as file:
            file.write(order_posting.text)
            file.close()
          logging.info(f"870 generated for the following PO# {po_number}")
        elif name['CardCode'] == 'C1040' or name['CardCode'] == 'C2063':
          if name['CardCode'] == 'C2063':
            for i in name['DocumentLines']:
              if i['TreeType'] != 'iSalesTree':
                result = AFFES_item_backorder(session_id, i['ItemCode'], i['WarehouseCode'], i['Quantity'])
                if result is None:
                  i['U_ZSPS_PackNote'] = 'backorder'
                elif result:
                  i['U_ZSPS_PackNote'] = 'fullorder'
          path_855 = "/edi_data/MichaelTodd/sftp/SAPB1/out/855"
          logging.info(f"855 generated for the following PO# {po_number}")
          file_name_855 = f"{name['BPLName']}-{name['CardCode']}-855_{name['DocNum']}_{po_number}.txt".replace(" ","")
          file_path_855 = os.path.join(path_855, file_name_855)
          with open(file_path_855, 'w', encoding="utf-8") as file:
            json.dump(name, file, indent=4)
        if name['DocTotal'] == 0:
          pass
        elif (name['CardCode'] == 'C2070' or name['CardCode'] == 'C2068' or name['CardCode'] == 'C1035' or name['CardCode'] == 'C2061' or name['CardCode'] == 'C1034' or name['CardCode'] == 'C2060' or name['CardCode'] == 'C2071' or name['CardCode'] == 'C2074' or name['CardCode'] == 'C2075' or name['CardCode'] == 'C2077') and name['DocTotal'] > 0:
          down_payment(session_id, name)
      else:
        logging.info(f"Unapproved Order generated for the following PO# {po_number}")

    elif order_posting.status_code == 400 and order_posting.json()['error']['code'] == -1116:
      data['Confirmed'] = 'tNO'
      data['U_SOApproval'] = 'Y'
      data['Comments'] = order_posting.json()['error']['message']['value']
      # print(data)
      # order_posting = requests.post(url, headers=header, cookies=cookies, json=data)
      if is_session_expired():
        login()
      order_posting = req('post', url, headers=header, cookies=cookies, json=data)

      if order_posting.status_code == 201:
        logging.info(f"Unapproved Order generated for the following PO# {po_number}")
      else:
          logging.error(f"Failed to generate unapproved order for following PO# {po_number}, Error: {order_posting.json()['error']['message']['value']}")
          raise Exception(f"\n{order_posting.status_code} Failed to generate order \nResposne: {order_posting.text}")

    elif order_posting.status_code==400 and order_posting.json()['error']['code']!=-1116:
      logging.error(f"\n{order_posting.status_code} Order data went wrong. \nDetails: {order_posting.json()['error']['message']['value']}")
      raise Exception(f"\n{order_posting.status_code} Order data went wrong. \nResposne: {order_posting.text}")
    else:
        logging.error(f"\n{order_posting.status_code} Something went wrong. \nDetails: {order_posting.json()['error']['message']['value']}")
        raise Exception(f"\n{order_posting.status_code} Something went wrong. \nResposne: {order_posting.text}")

def send_email(email_body):

  try:
    server = smtplib.SMTP(server_host, server_port)  
    
    server.ehlo()
    server.starttls()
    print('sender email is ', sender_email)
    server.login(sender_email, sender_pass)
    print("Login successful")
    
    msg = EmailMessage()
    msg['Subject'] = f'Failure in Process: {process_id}'
    msg['From'] = sender_email
    msg['To'] = receiver_email
    msg.set_content(email_body)

    server.send_message(msg)
    print("Email sent successfully")
      
  except smtplib.SMTPAuthenticationError as e:
    print(f"SMTP Authentication Error: {e}")
  except Exception as e:
    print(f"An error occurred: {e}")
  finally:
    server.quit()


if __name__ == '__main__':
  try:
    error_in_files = []

    if len(sys.argv)==6:
      # creds = sys.argv[1]
      input_dir = sys.argv[1]
      output_dir = sys.argv[2]
      error_dir = sys.argv[3]
      logs_dir = sys.argv[4]
      process_id = sys.argv[5]

      os.makedirs(logs_dir, exist_ok=True)

    else:
      print('No directory given')
    
    current_datetime = datetime.now()
    dt = current_datetime.strftime("%d-%m-%Y")
    log_file_name = f"PostOrderLogs_{dt}"
    log_path = os.path.join(logs_dir, f"{log_file_name}.log")

    logging.basicConfig(level=logging.DEBUG, format='%(asctime)s - %(levelname)s - %(message)s',
                    handlers=[logging.FileHandler(log_path, mode='a'),  # Log to a file
                    logging.StreamHandler()])  # Log to Console

    logging.info(f"ProcessID: {process_id}")
    if os.path.exists(input_dir):
      files = [f for f in os.listdir(input_dir) if os.path.isfile(os.path.join(input_dir, f))]

      login()
      datetime_now = datetime.now()
      login_at = datetime_now.strftime("%Y-%m-%d %H:%M:%S")

      for file_name in files:
        try:
          logging.info(f"------FileName: {file_name}------")
          file_path = os.path.join(input_dir, file_name)
          with open(file_path,'r') as file:
            order_data = file.read()
          # raise if there is any issue in json structure
          try:
            data = json.loads(order_data)
          except:
            raise ValueError(f"Something went wrong with json: {file_name}")
          
          data_i = remove_replace_me(data)
          data_i = remove_empty_dicts(data_i)

          orders_list = []
          if "Document" in data_i:
            orders_list = data_i["Document"]
          else:
             orders_list.append(data_i)

          error_in_orders_list = ''   # initializing here to save all errors in one error file for more than one orders in input json
          for order in orders_list:
            try:
              po_number = order['NumAtCard']
              post_order(order,file_name,output_dir)
            except Exception as e:
              error_in_orders_list += f"\n -PO Number = {po_number}\nError: {e}, \n"

          if error_in_orders_list:
            error_in_files.append(file_name)
            raise Exception(error_in_orders_list)

          # if file successfuly executed remove from the input directory
          if os.path.exists(file_path):
            os.remove(file_path)

        except Exception as e:
          status = None
          details = None

          # Check if e is a dictionary
          if isinstance(e, dict):
              status = e.get("status")
              details = e.get("message")
          else:
              details = str(e)

          collect_errors += f"    Error in File: {file_name} {details}\n----------------------------------\n"
          print(collect_errors)
          destination_path = os.path.join(error_dir, file_name)
          if status and status not in {401, 500, 501, 502, 503, 504}:
            if os.path.exists(file_path):
              shutil.move(file_path,destination_path)
          # create error file here

    if error_in_files:
      raise

  except Exception as e:
    error = f'Error in files: {error_in_files}'
    send_email(collect_errors)
    print(error)
    raise Exception(error)