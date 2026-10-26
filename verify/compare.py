"""Field-by-field comparison of substreams map_events output vs an independent IDL decode of
the same transactions fetched from Solana RPC (getTransaction, maxSupportedTransactionVersion=1)."""
import json,sys,collections
import idldec
from fetch import get
def S(x): return str(x)
def dec_of(tx,mint):
    m=tx['meta']
    for b in (m.get('preTokenBalances') or [])+(m.get('postTokenBalances') or []):
        if b['mint']==mint: return b['uiTokenAmount']['decimals']
    return -1
def ptype(c):
    return {'jup3YeL8QhtSx1e253b2FDvsMNC87fDrgQZivbrndc9':'earn','jupr81YtYssSyPt8jbnGuiWon5f6x9TcDEFxYe3Bdzi':'vault','jupZ4m2GqUCJ5iueMfzQf8khFfH31d4XAQt3RzCT9Vd':'dex','jupgfSgfuAXv4B6R2Uxu85Z1qdzgju79s6MfZekN6XS':'flashloan'}.get(c,'other')
def vt(ix):
    ix=ix or ''
    return 'perfect_dex' if ix.endswith('perfect_dex') else ('dex' if ix.endswith('_dex') else 'T1')
REG={v['vault_config']:v['vault_id'] for v in json.load(open('/workspace/jl/verify/vault_configs.json'))}
def expected(e,tx,group):
    f=e['fields'];a=e['accounts'];P=e['program'];N=e['event']
    vid=group.get(e['walk_index'],REG.get(a.get('vault_config',''),0) if P=='vaults' else 0)
    if P=='liquidity' and N=='LogOperate':
        return 'liquidityOperates',dict(protocol=f['user'],protocolType=ptype(e['caller']),mint=f['token'],supplyAmountRaw=S(f['supply_amount']),borrowAmountRaw=S(f['borrow_amount']),withdrawTo=f['withdraw_to'],borrowTo=f['borrow_to'],supplyExchangePrice=S(f['supply_exchange_price']),borrowExchangePrice=S(f['borrow_exchange_price']),decimals=dec_of(tx,f['token']))
    if P=='liquidity' and N=='LogUpdateExchangePrices':
        return 'liquidityExchangePrices',dict(mint=f['token'],supplyExchangePrice=S(f['supply_exchange_price']),borrowExchangePrice=S(f['borrow_exchange_price']),borrowRateBps=f['borrow_rate'],utilizationBps=f['utilization'])
    if P=='liquidity' and N=='LogCollectRevenue':
        return 'liquidityCollectRevenues',dict(mint=f['token'],revenueAmountRaw=S(f['revenue_amount']))
    if P=='liquidity' and N=='LogClaim':
        return 'liquidityClaims',dict(user=f['user'],mint=f['token'],recipient=f['recipient'],amountRaw=S(f['amount']))
    if P=='liquidity' and N=='LogUpdateRateDataV1':
        r=f['rate_data'];return 'liquidityRateDataV1',dict(mint=f['token'],kinkBps=S(r['kink']),rateAtUtilizationZeroBps=S(r['rate_at_utilization_zero']),rateAtUtilizationKinkBps=S(r['rate_at_utilization_kink']),rateAtUtilizationMaxBps=S(r['rate_at_utilization_max']))
    if P=='liquidity' and N=='LogUpdateRateDataV2':
        r=f['rate_data'];return 'liquidityRateDataV2',dict(mint=f['token'],kink1Bps=S(r['kink1']),kink2Bps=S(r['kink2']),rateAtUtilizationZeroBps=S(r['rate_at_utilization_zero']),rateAtUtilizationKink1Bps=S(r['rate_at_utilization_kink1']),rateAtUtilizationKink2Bps=S(r['rate_at_utilization_kink2']),rateAtUtilizationMaxBps=S(r['rate_at_utilization_max']))
    if P=='liquidity' and N=='LogUpdateTokenConfigs':
        r=f['token_config'];return 'liquidityTokenConfigs',dict(mint=r['token'],feeBps=S(r['fee']),maxUtilizationBps=S(r['max_utilization']))
    if P=='liquidity' and N=='LogUpdateUserSupplyConfigs':
        r=f['user_supply_config'];return 'liquidityUserSupplyConfigs',dict(protocol=f['user'],mint=f['token'],mode=r['mode'],expandPercent=S(r['expand_percent']),expandDuration=S(r['expand_duration']),baseWithdrawalLimit=S(r['base_withdrawal_limit']))
    if P=='liquidity' and N=='LogUpdateUserBorrowConfigs':
        r=f['user_borrow_config'];return 'liquidityUserBorrowConfigs',dict(protocol=f['user'],mint=f['token'],mode=r['mode'],expandPercent=S(r['expand_percent']),expandDuration=S(r['expand_duration']),baseDebtCeiling=S(r['base_debt_ceiling']),maxDebtCeiling=S(r['max_debt_ceiling']))
    if P=='vaults' and N=='LogInitVaultConfig':
        return 'vaultCreations',dict(vaultConfig=f['vault_config'],supplyToken=a.get('supply_token',''),borrowToken=a.get('borrow_token',''),oracle=a.get('oracle',''),vaultId=REG.get(f['vault_config'],-1))
    if P=='vaults' and N.startswith('LogUpdate') and N not in ('LogUpdateExchangePrices','LogUpdateAuths','LogUpdateAuthority','LogUpdateCoreSettings'):
        import re as _re
        p=_re.sub(r'(?<!^)(?=[A-Z])','_',N[9:]).lower();v=list(f.values())[0]
        return 'vaultParamUpdates',dict(vaultConfig=a.get('vault_config',''),param=p,value=S(v))
    if P=='dex' and N=='LogInitDex':
        return 'dexCreations',dict(dexId=f['dex_id'],token0=f['token_0'],token1=f['token_1'],smartCol=f['smart_col'],smartDebt=f['smart_debt'],fee=f['fee'],revenueCut=f['revenue_cut'],centerPrice=S(f['center_price']))
    if P=='lending' and N=='LogDeposit':
        return 'earnDeposits',dict(sender=f['sender'],receiver=f['receiver'],assetsRaw=S(f['assets']),sharesMintedRaw=S(f['shares_minted']),mint=a.get('mint',''),fTokenMint=a.get('f_token_mint',''),lending=a.get('lending',''))
    if P=='lending' and N=='LogWithdraw':
        return 'earnWithdraws',dict(sender=f['sender'],receiver=f['receiver'],owner=f['owner'],assetsRaw=S(f['assets']),sharesBurnedRaw=S(f['shares_burned']),mint=a.get('mint',''),fTokenMint=a.get('f_token_mint',''),lending=a.get('lending',''))
    if P=='lending' and N=='LogUpdateRates':
        return 'earnUpdateRates',dict(mint=a.get('mint',''),fTokenMint=a.get('f_token_mint',''),lending=a.get('lending',''),tokenExchangePrice=S(f['token_exchange_price']),liquidityExchangePrice=S(f['liquidity_exchange_price']))
    if P=='lending' and N=='LogRebalance':
        return 'earnRebalances',dict(assetsRaw=S(f['assets']),mint=a.get('mint',''))
    if P=='vaults' and N=='LogOperate':
        return 'vaultOperates',dict(vaultId=vid,vaultConfig=a.get('vault_config',''),supplyMint=a.get('supply_token',''),borrowMint=a.get('borrow_token',''),nftId=f['nft_id'],signer=f['signer'],to=f['to'],colChangeRaw=S(f['new_col']),debtChangeRaw=S(f['new_debt']),vaultType=vt(e['ix']))
    if P=='vaults' and N=='LogUserPosition':
        return 'vaultUserPositions',dict(user=f['user'],nftId=f['nft_id'],vaultId=f['vault_id'],positionMint=f['position_mint'],tick=f['tick'],colRaw=S(f['col']),borrowRaw=S(f['borrow']),vaultConfig=a.get('vault_config',''))
    if P=='vaults' and N=='LogLiquidate':
        return 'vaultLiquidates',dict(vaultId=vid,vaultConfig=a.get('vault_config',''),signer=f['signer'],to=f['to'],colAmountRaw=S(f['col_amount']),debtAmountRaw=S(f['debt_amount']),vaultType=vt(e['ix']))
    if P=='vaults' and N=='LogAbsorb':
        return 'vaultAbsorbs',dict(colAmountRaw=S(f['col_amount']),debtAmountRaw=S(f['debt_amount']))
    if P=='vaults' and N=='LogUpdateExchangePrices':
        return 'vaultExchangePrices',dict(vaultSupplyExchangePrice=S(f['vault_supply_exchange_price']),vaultBorrowExchangePrice=S(f['vault_borrow_exchange_price']),liquiditySupplyExchangePrice=S(f['liquidity_supply_exchange_price']),liquidityBorrowExchangePrice=S(f['liquidity_borrow_exchange_price']))
    if P=='vaults' and N=='LogRebalance':
        return 'vaultRebalances',dict(supplyAmountRaw=S(f['supply_amt']),borrowAmountRaw=S(f['borrow_amt']))
    if P=='vaults' and N=='LogClosePosition':
        return 'vaultClosePositions',dict(signer=f['signer'],positionId=f['position_id'],vaultId=f['vault_id'],positionMint=f['position_mint'])
    if P=='lending_reward_rate_model' and N=='LogStartRewards':
        return 'rewardsStarts',dict(mint=f['mint'],rewardAmountRaw=S(f['reward_amount']),duration=S(f['duration']),startTime=S(f['start_time']))
    if P=='lending_reward_rate_model' and N=='LogStopRewards':
        return 'rewardsStops',dict(mint=f['mint'])
    if P=='lending_reward_rate_model' and N=='LogTransitionedToNextRewards':
        return 'rewardsTransitions',dict(mint=f['mint'],startTime=S(f['start_time']),endTime=S(f['end_time']))
    if P=='dex' and N=='LogSwapInfo':
        return 'dexSwaps',dict(dexId=f['dex_id'],swap0To1=f['swap_0_to_1'],maker=f['maker'],to=f['to'],amountInRaw=S(f['amount_in']),amountOutRaw=S(f['amount_out']),feeAmountInRaw=S(f['fee_amount_in']),reservesToken0=S(f['reserves_token_0']),reservesToken1=S(f['reserves_token_1']),poolPrice=S(f['pool_price']),dex=a.get('dex',''),token0=a.get('token_0',''),token1=a.get('token_1',''))
    if P=='dex' and N.startswith(('LogDeposit','LogWithdraw','LogBorrow','LogPayback')):
        sh=f['shares'];t0=f.get('token_0_amt',f.get('amount_0'));t1=f.get('token_1_amt',f.get('amount_1'))
        return 'dexLiquidity',dict(dexId=f['dex_id'],sharesRaw=S(sh),token0AmountRaw=S(t0),token1AmountRaw=S(t1),user=f['user'],protocol=f['protocol'])
    if P=='dex' and N=='LogArbitrage':
        return 'dexArbitrages',dict(dexId=f['dex_id'],routing=S(f['routing']),amountOutRaw=S(f['amt_out']))
    return None,None
SKIP={'LogSwap','LogInitTick','LogInitTickIdLiquidation','LogInitBranch','LogInitTickHasDebtArray','LogLiquidateInfo'}
def norm(v):
    if isinstance(v,bool): return v
    return str(v)
def run(files,sigfile=None):
    got={}
    for fn in files:
        for l in open(fn):
            d=json.loads(l)['@data']
            for k,v in d.items():
                if isinstance(v,list):
                    for e in v:
                        if 'ctx' in e: got[(e['ctx']['txId'],e['ctx']['logIndex'] if 'logIndex' in e['ctx'] else 0)]=(k,e)
    txids=sorted({t for t,_ in got})
    if sigfile: txids=sorted(set(txids)|set(json.load(open(sigfile))))
    stats=collections.Counter();fields=collections.Counter();bad=[];seen=set()
    for t in txids:
        tx=get(t)
        if tx is None or tx['meta']['err']: stats['rpc_missing_or_failed']+=1;continue
        evs=idldec.decode_tx(tx)
        group={}
        for e in evs:
            if e['program']=='vaults' and 'vault_id' in e['fields']: group.setdefault(e['walk_index'],e['fields']['vault_id'])
        for e in evs:
            k,exp=expected(e,tx,group)
            if k is None:
                stats['skipped:'+e['event']]+=1;continue
            key=(t,e['log_index']);seen.add(key)
            if key not in got: stats['MISSING:'+k]+=1;bad.append(('missing',key,k));continue
            gk,g=got[key]
            if gk!=k: stats['WRONGTYPE']+=1;bad.append(('type',key,gk,k));continue
            ok=True
            for fk,fv in exp.items():
                dflt=False if isinstance(fv,bool) else (0 if isinstance(fv,int) else '')
                gv=g.get(fk,dflt)
                if norm(gv)!=norm(fv):
                    ok=False;bad.append(('field',key,k,fk,gv,fv))
                fields['checked']+=1
            stats[('OK:' if ok else 'BAD:')+k]+=1
            if g['ctx'].get('instruction','')!=(e['ix'] or ''): bad.append(('ixname',key,g['ctx'].get('instruction'),e['ix']))
    extra=[k for k in got if k not in seen]
    stats['extra_in_substreams']=len(extra)
    return stats,fields,bad,len(txids)
if __name__=='__main__':
    sig=None;files=[]
    for a in sys.argv[1:]:
        if a.startswith('--sigs='): sig=a[7:]
        else: files.append(a)
    st,fi,bad,n=run(files,sig)
    print('txs',n,'fields checked',fi['checked'])
    for k,v in sorted(st.items()): print(' ',k,v)
    print('problems',len(bad))
    for b in bad[:25]: print('  ',b)
