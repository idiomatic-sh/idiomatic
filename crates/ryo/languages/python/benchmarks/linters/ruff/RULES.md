# Ruff 0.16.5 rule check

**151 PASS / 262 FAIL / 413 TOTAL**

PASS means current fixture proof on sealed worker `934872341208654f3e8228aae360131703417cacd27d4dfbb8fbbcd437364bb5`. FAIL means that proof is missing; the reason is listed per rule. [Native143 receipts](results/native143-split-2026-11-02/summary.json) and [eight-rule supplement](results/native151-supplement-2026-11-02/summary.json) are separate runs, not a single 151-rule execution. The full 413-rule CI runner reports its own PASS/FAIL result and raw receipts.

| Rule | Result | Reason / evidence |
|---|---|---|
| ASYNC100 | FAIL | No current sealed-worker proof for [query](rules/ASYNC100/candidate.ryoql) |
| ASYNC105 | FAIL | No current sealed-worker proof for [query](rules/ASYNC105/candidate.ryoql) |
| ASYNC115 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 41/41 intended, 41 raw exact |
| ASYNC116 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 66/66 intended, 66 raw exact |
| ASYNC210 | FAIL | No current sealed-worker proof for [query](rules/ASYNC210/candidate.ryoql) |
| ASYNC220 | FAIL | No current sealed-worker proof for [query](rules/ASYNC220/candidate.ryoql) |
| ASYNC221 | FAIL | No current sealed-worker proof for [query](rules/ASYNC221/candidate.ryoql) |
| ASYNC222 | FAIL | No current sealed-worker proof for [query](rules/ASYNC222/candidate.ryoql) |
| ASYNC230 | FAIL | No current sealed-worker proof for [query](rules/ASYNC230/candidate.ryoql) |
| ASYNC251 | FAIL | No current sealed-worker proof for [query](rules/ASYNC251/candidate.ryoql) |
| B002 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 50/50 intended, 50 raw exact |
| B003 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 15/15 intended, 15 raw exact |
| B004 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 17/17 intended, 17 raw exact |
| B005 | FAIL | No current sealed-worker proof for [query](rules/B005/candidate.ryoql) |
| B006 | FAIL | No current sealed-worker proof for [query](rules/B006/candidate.ryoql) |
| B008 | FAIL | No current sealed-worker proof for [query](rules/B008/candidate.ryoql) |
| B009 | FAIL | No current sealed-worker proof for [query](rules/B009/candidate.ryoql) |
| B010 | FAIL | No current sealed-worker proof for [query](rules/B010/candidate.ryoql) |
| B012 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 20/20 intended, 20 raw exact |
| B013 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 16/16 intended, 16 raw exact |
| B014 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 18/18 intended, 18 raw exact |
| B015 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 19/19 intended, 19 raw exact |
| B016 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 24/24 intended, 24 raw exact |
| B017 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 20/20 intended, 20 raw exact |
| B018 | FAIL | No current sealed-worker proof for [query](rules/B018/candidate.ryoql) |
| B019 | FAIL | No current sealed-worker proof for [query](rules/B019/candidate.ryoql) |
| B020 | FAIL | No current sealed-worker proof for [query](rules/B020/candidate.ryoql) |
| B021 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 23/23 intended, 23 raw exact |
| B022 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 24/24 intended, 24 raw exact |
| B023 | FAIL | No current sealed-worker proof for [query](rules/B023/candidate.ryoql) |
| B025 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 17/17 intended, 17 raw exact |
| B026 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 18/18 intended, 18 raw exact |
| B029 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 14/14 intended, 14 raw exact |
| B030 | FAIL | No current sealed-worker proof for [query](rules/B030/candidate.ryoql) |
| B031 | FAIL | No query source |
| B032 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 20/20 intended, 20 raw exact |
| B033 | FAIL | No current sealed-worker proof for [query](rules/B033/candidate.ryoql) |
| B035 | FAIL | No current sealed-worker proof for [query](rules/B035/candidate.ryoql) |
| B039 | FAIL | No current sealed-worker proof for [query](rules/B039/candidate.ryoql) |
| BLE001 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 119/119 intended, 119 raw exact |
| C400 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 18/18 intended, 18 raw exact |
| C401 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 17/17 intended, 17 raw exact |
| C402 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 19/19 intended, 19 raw exact |
| C403 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 15/15 intended, 15 raw exact |
| C404 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 19/19 intended, 19 raw exact |
| C405 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 19/19 intended, 19 raw exact |
| C406 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 23/23 intended, 23 raw exact |
| C408 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 17/17 intended, 17 raw exact |
| C409 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 19/19 intended, 19 raw exact |
| C410 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 19/19 intended, 19 raw exact |
| C411 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 15/15 intended, 15 raw exact |
| C413 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 13/13 intended, 13 raw exact |
| C414 | FAIL | No current sealed-worker proof for [query](rules/C414/candidate.ryoql) |
| C415 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 16/16 intended, 16 raw exact |
| C417 | FAIL | No current sealed-worker proof for [query](rules/C417/candidate.ryoql) |
| C418 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 19/19 intended, 19 raw exact |
| C419 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 23/23 intended, 23 raw exact |
| D419 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 32/32 intended, 32 raw exact |
| DTZ001 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 14/14 intended, 14 raw exact |
| DTZ002 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 20/20 intended, 20 raw exact |
| DTZ003 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 20/20 intended, 20 raw exact |
| DTZ004 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 20/20 intended, 20 raw exact |
| DTZ005 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 24/24 intended, 24 raw exact |
| DTZ006 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 22/22 intended, 22 raw exact |
| DTZ007 | FAIL | No current sealed-worker proof for [query](rules/DTZ007/candidate.ryoql) |
| DTZ011 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 14/14 intended, 14 raw exact |
| DTZ012 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 14/14 intended, 14 raw exact |
| DTZ901 | FAIL | No current sealed-worker proof for [query](rules/DTZ901/candidate.ryoql) |
| E722 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 72/72 intended, 72 raw exact |
| E902 | FAIL | No query source |
| EXE001 | FAIL | No query source |
| EXE002 | FAIL | No query source |
| EXE004 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 21/21 intended, 21 raw exact |
| EXE005 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 26/26 intended, 26 raw exact |
| F401 | FAIL | No current sealed-worker proof for [query](rules/F401/candidate.ryoql) |
| F402 | FAIL | No current sealed-worker proof for [query](rules/F402/candidate.ryoql) |
| F404 | FAIL | No current sealed-worker proof for [query](rules/F404/candidate.ryoql) |
| F407 | FAIL | No current sealed-worker proof for [query](rules/F407/candidate.ryoql) |
| F501 | FAIL | No current sealed-worker proof for [query](rules/F501/candidate.ryoql) |
| F502 | FAIL | No current sealed-worker proof for [query](rules/F502/candidate.ryoql) |
| F503 | FAIL | No current sealed-worker proof for [query](rules/F503/candidate.ryoql) |
| F504 | FAIL | No current sealed-worker proof for [query](rules/F504/candidate.ryoql) |
| F505 | FAIL | No current sealed-worker proof for [query](rules/F505/candidate.ryoql) |
| F506 | FAIL | No current sealed-worker proof for [query](rules/F506/candidate.ryoql) |
| F507 | FAIL | No current sealed-worker proof for [query](rules/F507/candidate.ryoql) |
| F508 | FAIL | No current sealed-worker proof for [query](rules/F508/candidate.ryoql) |
| F509 | FAIL | No current sealed-worker proof for [query](rules/F509/candidate.ryoql) |
| F521 | FAIL | No current sealed-worker proof for [query](rules/F521/candidate.ryoql) |
| F522 | FAIL | No current sealed-worker proof for [query](rules/F522/candidate.ryoql) |
| F523 | FAIL | No current sealed-worker proof for [query](rules/F523/candidate.ryoql) |
| F524 | FAIL | No current sealed-worker proof for [query](rules/F524/candidate.ryoql) |
| F525 | FAIL | No current sealed-worker proof for [query](rules/F525/candidate.ryoql) |
| F541 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 21/21 intended, 21 raw exact |
| F601 | FAIL | No current sealed-worker proof for [query](rules/F601/candidate.ryoql) |
| F602 | FAIL | No current sealed-worker proof for [query](rules/F602/candidate.ryoql) |
| F621 | FAIL | No query source |
| F622 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 21/21 intended, 21 raw exact |
| F631 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 17/17 intended, 17 raw exact |
| F632 | FAIL | No current sealed-worker proof for [query](rules/F632/candidate.ryoql) |
| F633 | FAIL | No current sealed-worker proof for [query](rules/F633/candidate.ryoql) |
| F634 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 16/16 intended, 16 raw exact |
| F701 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 24/24 intended, 24 raw exact |
| F702 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 24/24 intended, 24 raw exact |
| F704 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 37/37 intended, 37 raw exact |
| F706 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 19/19 intended, 19 raw exact |
| F707 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 14/14 intended, 14 raw exact |
| F811 | FAIL | No current sealed-worker proof for [query](rules/F811/candidate.ryoql) |
| F821 | FAIL | No current sealed-worker proof for [query](rules/F821/candidate.ryoql) |
| F822 | FAIL | No current sealed-worker proof for [query](rules/F822/candidate.ryoql) |
| F823 | FAIL | No current sealed-worker proof for [query](rules/F823/candidate.ryoql) |
| F841 | FAIL | No current sealed-worker proof for [query](rules/F841/candidate.ryoql) |
| F842 | FAIL | No current sealed-worker proof for [query](rules/F842/candidate.ryoql) |
| F901 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 16/16 intended, 16 raw exact |
| FA100 | FAIL | No query source |
| FA102 | FAIL | No query source |
| FLY002 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 38/38 intended, 38 raw exact |
| FURB105 | FAIL | No current sealed-worker proof for [query](rules/FURB105/candidate.ryoql) |
| FURB122 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 98/98 intended, 85 raw exact |
| FURB129 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 131/131 intended, 97 raw exact |
| FURB132 | FAIL | No current sealed-worker proof for [query](rules/FURB132/candidate.ryoql) |
| FURB136 | FAIL | No current sealed-worker proof for [query](rules/FURB136/candidate.ryoql) |
| FURB157 | FAIL | No current sealed-worker proof for [query](rules/FURB157/candidate.ryoql) |
| FURB161 | FAIL | No current sealed-worker proof for [query](rules/FURB161/candidate.ryoql) |
| FURB162 | FAIL | No current sealed-worker proof for [query](rules/FURB162/candidate.ryoql) |
| FURB163 | FAIL | No current sealed-worker proof for [query](rules/FURB163/candidate.ryoql) |
| FURB166 | FAIL | No current sealed-worker proof for [query](rules/FURB166/candidate.ryoql) |
| FURB167 | FAIL | No current sealed-worker proof for [query](rules/FURB167/candidate.ryoql) |
| FURB168 | FAIL | No current sealed-worker proof for [query](rules/FURB168/candidate.ryoql) |
| FURB169 | FAIL | No current sealed-worker proof for [query](rules/FURB169/candidate.ryoql) |
| FURB177 | FAIL | No current sealed-worker proof for [query](rules/FURB177/candidate.ryoql) |
| FURB181 | FAIL | No current sealed-worker proof for [query](rules/FURB181/candidate.ryoql) |
| FURB188 | FAIL | No query source |
| FURB192 | FAIL | No current sealed-worker proof for [query](rules/FURB192/candidate.ryoql) |
| G010 | FAIL | No current sealed-worker proof for [query](rules/G010/candidate.ryoql) |
| G101 | FAIL | No current sealed-worker proof for [query](rules/G101/candidate.ryoql) |
| G201 | FAIL | No current sealed-worker proof for [query](rules/G201/candidate.ryoql) |
| G202 | FAIL | No current sealed-worker proof for [query](rules/G202/candidate.ryoql) |
| I001 | FAIL | No query source |
| INT001 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 16/16 intended, 16 raw exact |
| INT002 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 15/15 intended, 15 raw exact |
| INT003 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 15/15 intended, 15 raw exact |
| ISC004 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 26/26 intended, 26 raw exact |
| LOG001 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 22/22 intended, 22 raw exact |
| LOG002 | FAIL | No current sealed-worker proof for [query](rules/LOG002/candidate.ryoql) |
| LOG009 | FAIL | No current sealed-worker proof for [query](rules/LOG009/candidate.ryoql) |
| LOG014 | FAIL | No current sealed-worker proof for [query](rules/LOG014/candidate.ryoql) |
| LOG015 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 32/32 intended, 32 raw exact |
| N999 | FAIL | No query source |
| PERF101 | FAIL | No current sealed-worker proof for [query](rules/PERF101/candidate.ryoql) |
| PERF102 | FAIL | No current sealed-worker proof for [query](rules/PERF102/candidate.ryoql) |
| PERF402 | FAIL | No current sealed-worker proof for [query](rules/PERF402/candidate.ryoql) |
| PGH005 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 17/17 intended, 17 raw exact |
| PIE790 | FAIL | No current sealed-worker proof for [query](rules/PIE790/candidate.ryoql) |
| PIE794 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 30/30 intended, 30 raw exact |
| PIE796 | FAIL | No current sealed-worker proof for [query](rules/PIE796/candidate.ryoql) |
| PIE800 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 17/17 intended, 17 raw exact |
| PIE804 | FAIL | No current sealed-worker proof for [query](rules/PIE804/candidate.ryoql) |
| PIE807 | FAIL | No current sealed-worker proof for [query](rules/PIE807/candidate.ryoql) |
| PIE808 | FAIL | No current sealed-worker proof for [query](rules/PIE808/candidate.ryoql) |
| PIE810 | FAIL | No current sealed-worker proof for [query](rules/PIE810/candidate.ryoql) |
| PLC0105 | FAIL | No current sealed-worker proof for [query](rules/PLC0105/candidate.ryoql) |
| PLC0131 | FAIL | No current sealed-worker proof for [query](rules/PLC0131/candidate.ryoql) |
| PLC0132 | FAIL | No current sealed-worker proof for [query](rules/PLC0132/candidate.ryoql) |
| PLC0205 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 21/21 intended, 21 raw exact |
| PLC0206 | FAIL | No current sealed-worker proof for [query](rules/PLC0206/candidate.ryoql) |
| PLC0208 | FAIL | No current sealed-worker proof for [query](rules/PLC0208/candidate.ryoql) |
| PLC0414 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 26/26 intended, 26 raw exact |
| PLC3002 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 16/16 intended, 16 raw exact |
| PLE0100 | FAIL | No current sealed-worker proof for [query](rules/PLE0100/candidate.ryoql) |
| PLE0101 | FAIL | No current sealed-worker proof for [query](rules/PLE0101/candidate.ryoql) |
| PLE0115 | FAIL | No current sealed-worker proof for [query](rules/PLE0115/candidate.ryoql) |
| PLE0116 | FAIL | No query source |
| PLE0117 | FAIL | No current sealed-worker proof for [query](rules/PLE0117/candidate.ryoql) |
| PLE0118 | FAIL | No current sealed-worker proof for [query](rules/PLE0118/candidate.ryoql) |
| PLE0303 | FAIL | No current sealed-worker proof for [query](rules/PLE0303/candidate.ryoql) |
| PLE0304 | FAIL | No current sealed-worker proof for [query](rules/PLE0304/candidate.ryoql) |
| PLE0305 | FAIL | No current sealed-worker proof for [query](rules/PLE0305/candidate.ryoql) |
| PLE0307 | FAIL | No current sealed-worker proof for [query](rules/PLE0307/candidate.ryoql) |
| PLE0308 | FAIL | No current sealed-worker proof for [query](rules/PLE0308/candidate.ryoql) |
| PLE0309 | FAIL | No current sealed-worker proof for [query](rules/PLE0309/candidate.ryoql) |
| PLE0604 | FAIL | No current sealed-worker proof for [query](rules/PLE0604/candidate.ryoql) |
| PLE0605 | FAIL | No current sealed-worker proof for [query](rules/PLE0605/candidate.ryoql) |
| PLE0643 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 77/77 intended, 74 raw exact |
| PLE0704 | FAIL | No current sealed-worker proof for [query](rules/PLE0704/candidate.ryoql) |
| PLE1132 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 23/23 intended, 23 raw exact |
| PLE1142 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 49/49 intended, 48 raw exact |
| PLE1205 | FAIL | No current sealed-worker proof for [query](rules/PLE1205/candidate.ryoql) |
| PLE1206 | FAIL | No current sealed-worker proof for [query](rules/PLE1206/candidate.ryoql) |
| PLE1300 | FAIL | No query source |
| PLE1307 | FAIL | No current sealed-worker proof for [query](rules/PLE1307/candidate.ryoql) |
| PLE1310 | FAIL | No query source |
| PLE1507 | FAIL | No current sealed-worker proof for [query](rules/PLE1507/candidate.ryoql) |
| PLE1519 | FAIL | No current sealed-worker proof for [query](rules/PLE1519/candidate.ryoql) |
| PLE1520 | FAIL | No current sealed-worker proof for [query](rules/PLE1520/candidate.ryoql) |
| PLE1700 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 35/35 intended, 34 raw exact |
| PLE2502 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 13/13 intended, 13 raw exact |
| PLE2510 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 9/9 intended, 9 raw exact |
| PLE2512 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 6/6 intended, 6 raw exact |
| PLE2513 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 6/6 intended, 6 raw exact |
| PLE2514 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 6/6 intended, 6 raw exact |
| PLE2515 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 6/6 intended, 6 raw exact |
| PLR0124 | FAIL | No current sealed-worker proof for [query](rules/PLR0124/candidate.ryoql) |
| PLR0133 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 19/19 intended, 19 raw exact |
| PLR0206 | FAIL | No current sealed-worker proof for [query](rules/PLR0206/candidate.ryoql) |
| PLR0402 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 16/16 intended, 16 raw exact |
| PLR1704 | FAIL | No current sealed-worker proof for [query](rules/PLR1704/candidate.ryoql) |
| PLR1708 | FAIL | No current sealed-worker proof for [query](rules/PLR1708/candidate.ryoql) |
| PLR1711 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 27/27 intended, 27 raw exact |
| PLR1716 | FAIL | No current sealed-worker proof for [query](rules/PLR1716/candidate.ryoql) |
| PLR1722 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 12/12 intended, 12 raw exact |
| PLR1730 | FAIL | No current sealed-worker proof for [query](rules/PLR1730/candidate.ryoql) |
| PLR1733 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 94/94 intended, 94 raw exact |
| PLR1736 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 109/109 intended, 109 raw exact |
| PLR2044 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 50/50 intended, 50 raw exact |
| PLW0120 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 28/28 intended, 28 raw exact |
| PLW0127 | FAIL | No current sealed-worker proof for [query](rules/PLW0127/candidate.ryoql) |
| PLW0128 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 17/17 intended, 17 raw exact |
| PLW0129 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 21/21 intended, 21 raw exact |
| PLW0131 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 14/14 intended, 14 raw exact |
| PLW0133 | FAIL | No current sealed-worker proof for [query](rules/PLW0133/candidate.ryoql) |
| PLW0177 | FAIL | No current sealed-worker proof for [query](rules/PLW0177/candidate.ryoql) |
| PLW0211 | FAIL | No current sealed-worker proof for [query](rules/PLW0211/candidate.ryoql) |
| PLW0245 | FAIL | No current sealed-worker proof for [query](rules/PLW0245/candidate.ryoql) |
| PLW0406 | FAIL | No current sealed-worker proof for [query](rules/PLW0406/candidate.ryoql) |
| PLW0602 | FAIL | No current sealed-worker proof for [query](rules/PLW0602/candidate.ryoql) |
| PLW0604 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 21/21 intended, 21 raw exact |
| PLW0642 | FAIL | No current sealed-worker proof for [query](rules/PLW0642/candidate.ryoql) |
| PLW0711 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 16/16 intended, 16 raw exact |
| PLW1501 | FAIL | No current sealed-worker proof for [query](rules/PLW1501/candidate.ryoql) |
| PLW1507 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 19/19 intended, 19 raw exact |
| PLW1508 | FAIL | No current sealed-worker proof for [query](rules/PLW1508/candidate.ryoql) |
| PLW1509 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 17/17 intended, 17 raw exact |
| PLW1510 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 21/21 intended, 21 raw exact |
| PLW2101 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 34/34 intended, 33 raw exact |
| PT010 | FAIL | No current sealed-worker proof for [query](rules/PT010/candidate.ryoql) |
| PT014 | FAIL | No current sealed-worker proof for [query](rules/PT014/candidate.ryoql) |
| PT020 | FAIL | No current sealed-worker proof for [query](rules/PT020/candidate.ryoql) |
| PT025 | FAIL | No current sealed-worker proof for [query](rules/PT025/candidate.ryoql) |
| PT026 | FAIL | No current sealed-worker proof for [query](rules/PT026/candidate.ryoql) |
| PT031 | FAIL | No current sealed-worker proof for [query](rules/PT031/candidate.ryoql) |
| PTH124 | FAIL | No current sealed-worker proof for [query](rules/PTH124/candidate.ryoql) |
| PTH210 | FAIL | No current sealed-worker proof for [query](rules/PTH210/candidate.ryoql) |
| PYI001 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 34/34 intended, 34 raw exact |
| PYI002 | FAIL | No current sealed-worker proof for [query](rules/PYI002/candidate.ryoql) |
| PYI003 | FAIL | No current sealed-worker proof for [query](rules/PYI003/candidate.ryoql) |
| PYI004 | FAIL | No current sealed-worker proof for [query](rules/PYI004/candidate.ryoql) |
| PYI005 | FAIL | No current sealed-worker proof for [query](rules/PYI005/candidate.ryoql) |
| PYI006 | FAIL | No current sealed-worker proof for [query](rules/PYI006/candidate.ryoql) |
| PYI007 | FAIL | No current sealed-worker proof for [query](rules/PYI007/candidate.ryoql) |
| PYI008 | FAIL | No current sealed-worker proof for [query](rules/PYI008/candidate.ryoql) |
| PYI009 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 22/22 intended, 22 raw exact |
| PYI010 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 58/58 intended, 58 raw exact |
| PYI012 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 17/17 intended, 17 raw exact |
| PYI013 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 19/19 intended, 19 raw exact |
| PYI015 | FAIL | No current sealed-worker proof for [query](rules/PYI015/candidate.ryoql) |
| PYI016 | FAIL | No current sealed-worker proof for [query](rules/PYI016/candidate.ryoql) |
| PYI017 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 17/17 intended, 17 raw exact |
| PYI018 | FAIL | No current sealed-worker proof for [query](rules/PYI018/candidate.ryoql) |
| PYI019 | FAIL | No current sealed-worker proof for [query](rules/PYI019/candidate.ryoql) |
| PYI020 | FAIL | No current sealed-worker proof for [query](rules/PYI020/candidate.ryoql) |
| PYI025 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 18/18 intended, 18 raw exact |
| PYI026 | FAIL | No current sealed-worker proof for [query](rules/PYI026/candidate.ryoql) |
| PYI029 | FAIL | No current sealed-worker proof for [query](rules/PYI029/candidate.ryoql) |
| PYI030 | FAIL | No current sealed-worker proof for [query](rules/PYI030/candidate.ryoql) |
| PYI032 | FAIL | No current sealed-worker proof for [query](rules/PYI032/candidate.ryoql) |
| PYI033 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 18/18 intended, 18 raw exact |
| PYI034 | FAIL | No current sealed-worker proof for [query](rules/PYI034/candidate.ryoql) |
| PYI035 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 56/56 intended, 56 raw exact |
| PYI036 | FAIL | No current sealed-worker proof for [query](rules/PYI036/candidate.ryoql) |
| PYI041 | FAIL | No current sealed-worker proof for [query](rules/PYI041/candidate.ryoql) |
| PYI042 | FAIL | No current sealed-worker proof for [query](rules/PYI042/candidate.ryoql) |
| PYI043 | FAIL | No current sealed-worker proof for [query](rules/PYI043/candidate.ryoql) |
| PYI044 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 17/17 intended, 17 raw exact |
| PYI045 | FAIL | No current sealed-worker proof for [query](rules/PYI045/candidate.ryoql) |
| PYI046 | FAIL | No current sealed-worker proof for [query](rules/PYI046/candidate.ryoql) |
| PYI047 | FAIL | No current sealed-worker proof for [query](rules/PYI047/candidate.ryoql) |
| PYI048 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 21/21 intended, 21 raw exact |
| PYI049 | FAIL | No current sealed-worker proof for [query](rules/PYI049/candidate.ryoql) |
| PYI050 | FAIL | No current sealed-worker proof for [query](rules/PYI050/candidate.ryoql) |
| PYI052 | FAIL | No current sealed-worker proof for [query](rules/PYI052/candidate.ryoql) |
| PYI055 | FAIL | No current sealed-worker proof for [query](rules/PYI055/candidate.ryoql) |
| PYI057 | FAIL | No current sealed-worker proof for [query](rules/PYI057/candidate.ryoql) |
| PYI058 | FAIL | No current sealed-worker proof for [query](rules/PYI058/candidate.ryoql) |
| PYI059 | FAIL | No current sealed-worker proof for [query](rules/PYI059/candidate.ryoql) |
| PYI061 | FAIL | No current sealed-worker proof for [query](rules/PYI061/candidate.ryoql) |
| PYI062 | FAIL | No current sealed-worker proof for [query](rules/PYI062/candidate.ryoql) |
| PYI063 | FAIL | No current sealed-worker proof for [query](rules/PYI063/candidate.ryoql) |
| PYI064 | FAIL | No current sealed-worker proof for [query](rules/PYI064/candidate.ryoql) |
| PYI066 | FAIL | No current sealed-worker proof for [query](rules/PYI066/candidate.ryoql) |
| RET501 | FAIL | No current sealed-worker proof for [query](rules/RET501/candidate.ryoql) |
| RUF007 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 64/64 intended, 60 raw exact |
| RUF008 | FAIL | No current sealed-worker proof for [query](rules/RUF008/candidate.ryoql) |
| RUF009 | FAIL | No query source |
| RUF010 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 10/10 intended, 10 raw exact |
| RUF012 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 8/8 intended, 8 raw exact |
| RUF013 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 12/12 intended, 12 raw exact |
| RUF015 | FAIL | No current sealed-worker proof for [query](rules/RUF015/candidate.ryoql) |
| RUF016 | FAIL | No current sealed-worker proof for [query](rules/RUF016/candidate.ryoql) |
| RUF017 | FAIL | No current sealed-worker proof for [query](rules/RUF017/candidate.ryoql) |
| RUF018 | FAIL | No current sealed-worker proof for [query](rules/RUF018/candidate.ryoql) |
| RUF019 | FAIL | No current sealed-worker proof for [query](rules/RUF019/candidate.ryoql) |
| RUF020 | FAIL | No current sealed-worker proof for [query](rules/RUF020/candidate.ryoql) |
| RUF022 | FAIL | No query source |
| RUF023 | FAIL | No query source |
| RUF024 | FAIL | No current sealed-worker proof for [query](rules/RUF024/candidate.ryoql) |
| RUF026 | FAIL | No current sealed-worker proof for [query](rules/RUF026/candidate.ryoql) |
| RUF028 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 97/97 intended, 93 raw exact |
| RUF030 | FAIL | No current sealed-worker proof for [query](rules/RUF030/candidate.ryoql) |
| RUF032 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 19/19 intended, 19 raw exact |
| RUF033 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 14/14 intended, 14 raw exact |
| RUF034 | FAIL | No current sealed-worker proof for [query](rules/RUF034/candidate.ryoql) |
| RUF040 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 19/19 intended, 19 raw exact |
| RUF041 | FAIL | No current sealed-worker proof for [query](rules/RUF041/candidate.ryoql) |
| RUF046 | PASS | [sealed fixture proof](results/native151-supplement-2026-11-02/RUF046/result.json): 29/29 intended and raw exact |
| RUF048 | FAIL | No current sealed-worker proof for [query](rules/RUF048/candidate.ryoql) |
| RUF049 | FAIL | No current sealed-worker proof for [query](rules/RUF049/candidate.ryoql) |
| RUF051 | FAIL | No current sealed-worker proof for [query](rules/RUF051/candidate.ryoql) |
| RUF053 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 15/15 intended, 15 raw exact |
| RUF057 | PASS | [sealed fixture proof](results/native151-supplement-2026-11-02/RUF057/result.json): 28/28 intended and raw exact |
| RUF058 | FAIL | No current sealed-worker proof for [query](rules/RUF058/candidate.ryoql) |
| RUF059 | FAIL | No current sealed-worker proof for [query](rules/RUF059/candidate.ryoql) |
| RUF063 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 27/27 intended, 27 raw exact |
| RUF068 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 26/26 intended, 26 raw exact |
| RUF100 | FAIL | No query source |
| RUF101 | FAIL | No query source |
| RUF200 | FAIL | No query source |
| S102 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 28/28 intended, 28 raw exact |
| S110 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 14/14 intended, 14 raw exact |
| S112 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 14/14 intended, 14 raw exact |
| SIM101 | FAIL | No current sealed-worker proof for [query](rules/SIM101/candidate.ryoql) |
| SIM102 | FAIL | No current sealed-worker proof for [query](rules/SIM102/candidate.ryoql) |
| SIM103 | FAIL | No current sealed-worker proof for [query](rules/SIM103/candidate.ryoql) |
| SIM107 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 11/11 intended, 11 raw exact |
| SIM113 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 46/46 intended, 44 raw exact |
| SIM114 | FAIL | No current sealed-worker proof for [query](rules/SIM114/candidate.ryoql) |
| SIM115 | FAIL | No query source |
| SIM117 | FAIL | No current sealed-worker proof for [query](rules/SIM117/candidate.ryoql) |
| SIM118 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 20/20 intended, 20 raw exact |
| SIM201 | FAIL | No current sealed-worker proof for [query](rules/SIM201/candidate.ryoql) |
| SIM202 | FAIL | No current sealed-worker proof for [query](rules/SIM202/candidate.ryoql) |
| SIM208 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 16/16 intended, 16 raw exact |
| SIM210 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 17/17 intended, 17 raw exact |
| SIM211 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 17/17 intended, 17 raw exact |
| SIM220 | FAIL | No current sealed-worker proof for [query](rules/SIM220/candidate.ryoql) |
| SIM221 | FAIL | No current sealed-worker proof for [query](rules/SIM221/candidate.ryoql) |
| SIM222 | FAIL | No current sealed-worker proof for [query](rules/SIM222/candidate.ryoql) |
| SIM223 | FAIL | No current sealed-worker proof for [query](rules/SIM223/candidate.ryoql) |
| SIM401 | FAIL | No query source |
| SIM905 | FAIL | No current sealed-worker proof for [query](rules/SIM905/candidate.ryoql) |
| SIM911 | FAIL | No current sealed-worker proof for [query](rules/SIM911/candidate.ryoql) |
| T100 | FAIL | No current sealed-worker proof for [query](rules/T100/candidate.ryoql) |
| TC004 | FAIL | No query source |
| TC005 | FAIL | No current sealed-worker proof for [query](rules/TC005/candidate.ryoql) |
| TC007 | FAIL | No query source |
| TC010 | PASS | [sealed fixture proof](results/native151-supplement-2026-11-02/TC010/result.json): 19/19 intended and raw exact |
| TRY002 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 12/12 intended, 12 raw exact |
| TRY004 | FAIL | No current sealed-worker proof for [query](rules/TRY004/candidate.ryoql) |
| TRY201 | FAIL | No current sealed-worker proof for [query](rules/TRY201/candidate.ryoql) |
| TRY203 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 25/25 intended, 25 raw exact |
| TRY401 | FAIL | No current sealed-worker proof for [query](rules/TRY401/candidate.ryoql) |
| UP001 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 14/14 intended, 14 raw exact |
| UP003 | FAIL | No current sealed-worker proof for [query](rules/UP003/candidate.ryoql) |
| UP004 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 11/11 intended, 11 raw exact |
| UP005 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 31/31 intended, 31 raw exact |
| UP006 | FAIL | No current sealed-worker proof for [query](rules/UP006/candidate.ryoql) |
| UP007 | FAIL | No current sealed-worker proof for [query](rules/UP007/candidate.ryoql) |
| UP008 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 35/35 intended, 31 raw exact |
| UP009 | FAIL | No current sealed-worker proof for [query](rules/UP009/candidate.ryoql) |
| UP010 | FAIL | No current sealed-worker proof for [query](rules/UP010/candidate.ryoql) |
| UP011 | FAIL | No current sealed-worker proof for [query](rules/UP011/candidate.ryoql) |
| UP012 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 50/50 intended, 42 raw exact |
| UP014 | PASS | [sealed fixture proof](results/native151-supplement-2026-11-02/UP014/result.json): 22/22 intended and raw exact |
| UP017 | FAIL | No current sealed-worker proof for [query](rules/UP017/candidate.ryoql) |
| UP018 | FAIL | No current sealed-worker proof for [query](rules/UP018/candidate.ryoql) |
| UP019 | FAIL | No current sealed-worker proof for [query](rules/UP019/candidate.ryoql) |
| UP020 | FAIL | No current sealed-worker proof for [query](rules/UP020/candidate.ryoql) |
| UP021 | FAIL | No current sealed-worker proof for [query](rules/UP021/candidate.ryoql) |
| UP022 | FAIL | No current sealed-worker proof for [query](rules/UP022/candidate.ryoql) |
| UP023 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 25/25 intended, 25 raw exact |
| UP024 | FAIL | No current sealed-worker proof for [query](rules/UP024/candidate.ryoql) |
| UP025 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 23/23 intended, 19 raw exact |
| UP026 | FAIL | No current sealed-worker proof for [query](rules/UP026/candidate.ryoql) |
| UP028 | FAIL | No current sealed-worker proof for [query](rules/UP028/candidate.ryoql) |
| UP029 | FAIL | No current sealed-worker proof for [query](rules/UP029/candidate.ryoql) |
| UP030 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 36/36 intended, 36 raw exact |
| UP031 | FAIL | No query source |
| UP032 | FAIL | No query source |
| UP033 | FAIL | No current sealed-worker proof for [query](rules/UP033/candidate.ryoql) |
| UP034 | FAIL | No query source |
| UP035 | FAIL | No query source |
| UP036 | FAIL | No query source |
| UP037 | PASS | [sealed fixture proof](results/native151-supplement-2026-11-02/UP037/result.json): 19/19 intended and raw exact |
| UP039 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 17/17 intended, 17 raw exact |
| UP040 | PASS | [sealed fixture proof](results/native151-supplement-2026-11-02/UP040/result.json): 21/21 intended and raw exact |
| UP041 | FAIL | No current sealed-worker proof for [query](rules/UP041/candidate.ryoql) |
| UP043 | PASS | [sealed fixture proof](results/native151-supplement-2026-11-02/UP043/result.json): 18/18 intended and raw exact |
| UP044 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 29/29 intended, 29 raw exact |
| UP045 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 33/33 intended, 32 raw exact |
| UP046 | FAIL | No query source |
| UP047 | FAIL | No query source |
| UP049 | PASS | [sealed fixture proof](results/native151-supplement-2026-11-02/UP049/result.json): 19/19 intended and raw exact |
| UP050 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 18/18 intended, 17 raw exact |
| W605 | PASS | [sealed fixture proof](results/native143-split-2026-11-02/summary.json): 20/20 intended, 20 raw exact |
| YTT101 | FAIL | No current sealed-worker proof for [query](rules/YTT101/candidate.ryoql) |
| YTT102 | FAIL | No current sealed-worker proof for [query](rules/YTT102/candidate.ryoql) |
| YTT103 | FAIL | No current sealed-worker proof for [query](rules/YTT103/candidate.ryoql) |
| YTT201 | FAIL | No current sealed-worker proof for [query](rules/YTT201/candidate.ryoql) |
| YTT202 | FAIL | No current sealed-worker proof for [query](rules/YTT202/candidate.ryoql) |
| YTT203 | FAIL | No current sealed-worker proof for [query](rules/YTT203/candidate.ryoql) |
| YTT204 | FAIL | No current sealed-worker proof for [query](rules/YTT204/candidate.ryoql) |
| YTT301 | FAIL | No current sealed-worker proof for [query](rules/YTT301/candidate.ryoql) |
| YTT302 | FAIL | No current sealed-worker proof for [query](rules/YTT302/candidate.ryoql) |
| YTT303 | FAIL | No current sealed-worker proof for [query](rules/YTT303/candidate.ryoql) |
