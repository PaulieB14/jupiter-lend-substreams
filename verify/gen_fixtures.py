import json,os,sys,collections,shutil
import idldec,compare
OUT='../tests/fixtures'
want=collections.defaultdict(list);os.makedirs(OUT,exist_ok=True)
for fn in os.listdir('txs'):
    t=json.load(open('txs/'+fn))
    if not t or t['meta']['err']: continue
    evs=idldec.decode_tx(t)
    for e in evs: want[(e['program'],e['event'],e['ix'])].append(fn)
chosen=set()
for k,v in sorted(want.items()):
    for fn in sorted(v)[:2]: chosen.add(fn)
n=0
for fn in sorted(chosen):
    t=json.load(open('txs/'+fn));evs=idldec.decode_tx(t)
    group={}
    for e in evs:
        if e['program']=='vaults' and 'vault_id' in e['fields']: group.setdefault(e['walk_index'],e['fields']['vault_id'])
    exp=[]
    for e in evs:
        k,f=compare.expected(e,t,group)
        if k is None or k in ('vaultExchangePrices','vaultRebalances','vaultClosePositions','rewardsTransitions','liquidityCollectRevenues','liquidityClaims','earnRebalances','dexArbitrages'): continue
        f=dict(f);f['instruction']=e['ix'] or ''
        f={a:(str(b).lower() if isinstance(b,bool) else str(b)) for a,b in f.items()}
        exp.append(dict(list=k,log_index=e['log_index'],fields=f))
    sig=fn[:-5];json.dump(t,open(f'{OUT}/{sig[:16]}.tx.json','w'));json.dump(exp,open(f'{OUT}/{sig[:16]}.expected.json','w'),indent=0);n+=1
print(n,'fixtures',sorted(set((k[0],k[1]) for k in want)))
