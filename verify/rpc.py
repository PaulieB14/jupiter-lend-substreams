import json,urllib.request,sys,time
R="https://api.mainnet-beta.solana.com"
def rpc(m,p):
    for i in range(6):
        try:
            r=urllib.request.urlopen(urllib.request.Request(R,json.dumps({"jsonrpc":"2.0","id":1,"method":m,"params":p}).encode(),{"content-type":"application/json"}),timeout=60)
            j=json.load(r)
            if 'error' in j: raise Exception(j['error'])
            return j['result']
        except Exception as e:
            time.sleep(2+i*3); err=e
    raise err
def oldest(addr,maxpages=200):
    before=None;last=None
    for _ in range(maxpages):
        p={"limit":1000}
        if before:p["before"]=before
        s=rpc("getSignaturesForAddress",[addr,p])
        if not s: break
        last=s[-1];before=last['signature']
        if len(s)<1000: break
    return last
