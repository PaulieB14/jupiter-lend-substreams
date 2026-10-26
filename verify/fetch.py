import json,os,time,sys
from rpc import rpc
os.makedirs('txs',exist_ok=True)
def get(sig):
    fn=f'txs/{sig}.json'
    if not os.path.exists(fn):
        t=rpc('getTransaction',[sig,{'encoding':'json','maxSupportedTransactionVersion':1}])
        json.dump(t,open(fn,'w'));time.sleep(0.12)
    return json.load(open(fn))
