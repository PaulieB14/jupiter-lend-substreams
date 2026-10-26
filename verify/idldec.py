"""Independent IDL-driven decoder used as the verification oracle (does not share code with the Rust decoder)."""
import json,hashlib,base64,base58,re,os
IDL_DIR=os.path.join(os.path.dirname(os.path.abspath(__file__)),'..','idl')
PROGS={}
for f in ['liquidity','lending','vaults','lending_reward_rate_model','flashloan','dex']:
    d=json.load(open(f'{IDL_DIR}/{f}.json'))
    types={t['name']:t for t in d.get('types',[])}
    evs={bytes(e['discriminator']):e['name'] for e in d.get('events',[])}
    ixs={bytes(i['discriminator']):i for i in d['instructions']}
    PROGS[d['address']]=dict(name=f,types=types,events=evs,ixs=ixs)
def rd(t,b,o,types):
    if isinstance(t,str):
        if t=='pubkey': return base58.b58encode(b[o:o+32]).decode(),o+32
        if t=='bool': return b[o]!=0,o+1
        m=re.match(r'([ui])(\d+)',t)
        if m:
            n=int(m.group(2))//8; return int.from_bytes(b[o:o+n],'little',signed=m.group(1)=='i'),o+n
        if t=='bytes':
            n=int.from_bytes(b[o:o+4],'little'); return b[o+4:o+4+n].hex(),o+4+n
        raise Exception(t)
    if 'vec' in t:
        n=int.from_bytes(b[o:o+4],'little');o+=4;out=[]
        for _ in range(n):
            v,o=rd(t['vec'],b,o,types);out.append(v)
        return out,o
    if 'array' in t:
        et,n=t['array'];out=[]
        for _ in range(n):
            v,o=rd(et,b,o,types);out.append(v)
        return out,o
    if 'option' in t:
        if b[o]==0: return None,o+1
        return rd(t['option'],b,o+1,types)
    if 'defined' in t:
        td=types[t['defined']['name']]['type']
        if td['kind']=='struct':
            r={}
            for f in td['fields']:
                r[f['name']],o=rd(f['type'],b,o,types)
            return r,o
        if td['kind']=='enum':
            v=td['variants'][b[o]];return v['name'],o+1
    raise Exception(t)
def decode_tx(tx):
    """returns list of events with emitting program, instruction walk index, ix name, accounts"""
    m=tx['meta'];msg=tx['transaction']['message']
    keys=msg['accountKeys']+m.get('loadedAddresses',{}).get('writable',[])+m.get('loadedAddresses',{}).get('readonly',[])
    inner={ii['index']:ii['instructions'] for ii in m.get('innerInstructions') or []}
    walk=[]
    for i,ix in enumerate(msg['instructions']):
        walk.append(ix)
        walk+=inner.get(i,[])
    stack=[];k=-1;events=[];nlog=0
    for line in m['logMessages']:
        mm=re.match(r'Program (\w+) invoke \[(\d+)\]',line)
        if mm:
            k+=1
            # instructions without invoke logs (precompiles) are skipped by matching program ids
            while k<len(walk) and keys[walk[k]['programIdIndex']]!=mm.group(1): k+=1
            stack.append(k);continue
        if re.match(r'Program \w+ (success|failed)',line):
            stack.pop();continue
        if line.startswith('Program data: ') and stack:
            wi=stack[-1];ix=walk[wi];pid=keys[ix['programIdIndex']]
            if pid not in PROGS: continue
            P=PROGS[pid];raw=base64.b64decode(line[14:]);ordinal=nlog;nlog+=1
            name=P['events'].get(raw[:8])
            if not name: continue
            f,_=rd({'defined':{'name':name}},raw,8,P['types'])
            idata=base58.b58decode(ix['data']);ixd=P['ixs'].get(idata[:8])
            accs={}
            if ixd:
                for j,a in enumerate(ixd['accounts']):
                    if j<len(ix['accounts']): accs[a['name']]=keys[ix['accounts'][j]]
            parent=None
            if len(stack)>1: parent=keys[walk[stack[-2]]['programIdIndex']]
            events.append(dict(log_index=ordinal,program=P['name'],event=name,fields=f,walk_index=wi,ix=ixd['name'] if ixd else None,accounts=accs,caller=parent))
    return events
