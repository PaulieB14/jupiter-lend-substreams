import json,base64,struct,hashlib,urllib.request
from rpc import rpc
import base58
def first_eps(fn):
    o={}
    for l in open(fn):
        d=json.loads(l)['@data']
        for e in d.get('liquidityOperates',[]):
            m=e['mint'];t=int(e['ctx']['blockTime'])
            if m not in o: o[m]=(t,int(e['supplyExchangePrice']),int(e['borrowExchangePrice']))
    return o
a=first_eps('d0.jsonl');b=first_eps('d1.jsonl')
disc=hashlib.sha256(b'account:TokenReserve').digest()[:8]
r=rpc('getProgramAccounts',['jupeiUmn818Jg1ekPURTpr4mFo29p46vygyykFJ3wZC',{'encoding':'base64','filters':[{'memcmp':{'offset':0,'bytes':base58.b58encode(disc).decode()}}]}])
res={}
for x in r:
    d=base64.b64decode(x['account']['data'][0])[8:]
    mint=base58.b58encode(d[:32]).decode()
    br,fee,ut,lu,sep,bep,mu,tswi,tsif,tbwi,tbif=struct.unpack_from('<HHHQQQHQQQQ',d,64)
    res[mint]=dict(fee=fee,sep=sep,bep=bep,tswi=tswi,tsif=tsif,tbwi=tbwi,tbif=tbif,br=br,ut=ut)
ids=','.join(res)
px=json.load(urllib.request.urlopen(urllib.request.Request('https://lite-api.jup.ag/price/v3?ids='+ids,headers={'User-Agent':'x'})))
dec={}
for fn in ['d0.jsonl','d1.jsonl']:
    for l in open(fn):
        for t in json.loads(l)['@data'].get('tokens',[]): dec[t['mint']]=t.get('decimals',0)
tot_b=tot_rev=tot_sup=0;rows=[]
for m,v in res.items():
    if m not in a or m not in b or m not in px: continue
    dt=b[m][0]-a[m][0]; k=86400/dt
    gb=(b[m][2]/a[m][2]-1)*k; gs=(b[m][1]/a[m][1]-1)*k
    p=px[m]['usdPrice']; dd=dec.get(m,px[m].get('decimals',0))
    borrow_usd=v['tbwi']*v['bep']/1e12/10**dd*p
    supply_usd=v['tswi']*v['sep']/1e12/10**dd*p
    bi=borrow_usd*gb; si=supply_usd*gs
    tot_b+=bi;tot_sup+=si;tot_rev+=bi-si
    rows.append((m[:6],round(borrow_usd/1e6,2),round(gb*365*100,2),round(bi),round(si),v['fee']))
rows.sort(key=lambda x:-x[3])
for r_ in rows[:12]: print(r_)
print('borrow interest/day $%.0f  supplier interest/day $%.0f  protocol revenue (diff) $%.0f'%(tot_b,tot_sup,tot_rev))
