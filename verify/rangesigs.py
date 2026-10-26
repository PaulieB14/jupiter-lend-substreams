import sys,json
from rpc import rpc
progs=['jupeiUmn818Jg1ekPURTpr4mFo29p46vygyykFJ3wZC','jup3YeL8QhtSx1e253b2FDvsMNC87fDrgQZivbrndc9','jupr81YtYssSyPt8jbnGuiWon5f6x9TcDEFxYe3Bdzi','jup7TthsMgcR9Y3L277b8Eo9uboVSmu1utkuXHNUKar','jupZ4m2GqUCJ5iueMfzQf8khFfH31d4XAQt3RzCT9Vd','jupgfSgfuAXv4B6R2Uxu85Z1qdzgju79s6MfZekN6XS']
lo,hi=int(sys.argv[1]),int(sys.argv[2])
ok=set();allsig=set()
for p in progs:
    before=None
    while True:
        prm={'limit':1000}
        if before: prm['before']=before
        s=rpc('getSignaturesForAddress',[p,prm])
        if not s: break
        for x in s:
            if lo<=x['slot']<hi:
                allsig.add(x['signature'])
                if not x['err']: ok.add(x['signature'])
        before=s[-1]['signature']
        if s[-1]['slot']<lo: break
json.dump(sorted(ok),open(f'sigs_{lo}_{hi}.json','w'))
print(len(ok),len(allsig))
