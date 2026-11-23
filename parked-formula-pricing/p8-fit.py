"""Generate neutral nanosecond prices for executor work and formula classes."""
from pathlib import Path
from collections import defaultdict
from statistics import median
from itertools import groupby
import csv
from p8_stream import trace
root=Path(__file__).resolve().parents[1]/'runs/pass8'
def tsv(path):
    with path.open() as f: return list(csv.DictReader(f,delimiter='\t'))
def positive_square_fit(observations, fixed):
    # Fit two nonnegative slopes. Check both coordinate boundaries as well.
    xx=xy=yy=xs=ys=0.
    for x,y,n,sec in observations:
        t=max(0.,sec*1e9/n-fixed)
        xx+=n*x*x; xy+=n*x*y; yy+=n*y*y
        xs+=n*x*t; ys+=n*y*t
    candidates=[(0.,0.),(xs/xx if xx else 0.,0.),(0.,ys/yy if yy else 0.)]
    det=xx*yy-xy*xy
    if det>0:
        a=(xs*yy-ys*xy)/det;b=(ys*xx-xs*xy)/det
        if a>=0 and b>=0:candidates.append((a,b))
    return min(candidates,key=lambda prices:sum(n*(sec*1e9/n-fixed-prices[0]*x-prices[1]*y)**2 for x,y,n,sec in observations))
fits=[]
for c in ('calpy','calts','calrs'):
    v=[]
    with trace(root/f'measure-p8meter-light-{c}.tsv') as f:
        for line in f:
            tag,*fields=line.rstrip().split('\t')
            if tag=='Q':
                x,y,n,sec=fields;v.append((int(x),int(y),int(n),float(sec)))
    if not v:continue
    zero=[(n,sec) for x,y,n,sec in v if not x and not y]
    fixed=1e9*sum(sec for _,sec in zero)/sum(n for n,_ in zero) if zero else 0.
    step,row=positive_square_fit(v,fixed)
    fits.append((c,fixed,step,row))
assert fits,'no successful empty neutral asks'
with (root/'empty-work-fit.tsv').open('w') as f:
    w=csv.writer(f,delimiter='\t',lineterminator='\n')
    w.writerow(('fixture','fixed_ns','step_ns','output_row_ns'));w.writerows(fits)
def classes(name,key,value):
    m=defaultdict(list)
    for v in tsv(root/name):m[v[key]].append(float(v[value]))
    return {k:max(1,round(median(v))) for k,v in m.items()}
ops=classes('operation-prices.tsv','operation','ns_per_call')
reads=classes('calibration.tsv','class','ns_per_observation')
steps=reads.pop('executor step')
lines=['//! Prices in nanoseconds from the generated neutral workload.','use super::cost::Family;']
fallback=max(ops.values())
for k in ('logic','cell','tree','probe','text','count'):
    lines.append(f'pub(super) const {k.upper()}:u64 = {ops.get(k,fallback)};')
# A lowered step includes its average observed output on the neutral workload.
lines.append(f'pub(super) const STEP:u64 = {steps};')
for col,k in enumerate(('fixed','paid_step','output_row'),1):
    lines.append(f'pub(super) const {k.upper()}:u64 = {max(0,round(median(v[col] for v in fits)))};')
variants={'logic':'Logic','shape':'Shape','text':'Text','fact':'Fact','parent':'Parent','child':'Child','exclude':'Exclusion','row':'Row','schedule':'Schedule','count':'Count','sets':'Sets','filename':'Filename','project':'Project'}
assert reads
lines.extend(['pub(super) fn provider(class:Family)->u64 {','    match class {'])
for k,v in sorted(reads.items()):
    lines.append(f'        Family::{variants[k]} => {v},')
lines.extend([f'        _ => {max(reads.values())},','    }','}'])
(root/'neutral-prices.rs').write_text('\n'.join(lines)+'\n')

