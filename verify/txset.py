import json,sys
def txs(fn):
    ss=set()
    for l in open(fn):
        d=json.loads(l)['@data']
        for k,v in d.items():
            if isinstance(v,list):
                for e in v:
                    if 'ctx' in e: ss.add(e['ctx']['txId'])
    return ss
if __name__=='__main__':
    a=txs(sys.argv[1]);r=set(json.load(open(sys.argv[2])))
    print('substreams',len(a),'rpc',len(r),'missing',len(r-a),'extra',len(a-r))
    for s in sorted(r-a)[:10]: print(' missing',s)
