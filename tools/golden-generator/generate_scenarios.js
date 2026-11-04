const fs = require('fs');
const path = require('path');
const { ReferenceEnvelopeEngine } = require('./reference_calculator');

const fixturesDir = path.resolve(__dirname, '../../fixtures/budgets');

// Scenario 1: Multi-Month Rollover Scenario
function generateMultiMonthRollover() {
  const engine = new ReferenceEnvelopeEngine();

  const groups = [
    { id: 'grp_income', name: 'Income', is_income: true },
    { id: 'grp_bills', name: 'Fixed Bills', is_income: false },
    { id: 'grp_everyday', name: 'Everyday', is_income: false },
  ];
  const categories = [
    { id: 'cat_salary', group_id: 'grp_income', name: 'Salary', is_income: true },
    { id: 'cat_rent', group_id: 'grp_bills', name: 'Rent', is_income: false },
    { id: 'cat_util', group_id: 'grp_bills', name: 'Utilities', is_income: false },
    { id: 'cat_groc', group_id: 'grp_everyday', name: 'Groceries', is_income: false },
    { id: 'cat_dine', group_id: 'grp_everyday', name: 'Dining Out', is_income: false },
  ];

  groups.forEach(g => engine.registerGroup(g));
  categories.forEach(c => engine.registerCategory(c));

  // --- Month 1: 2026-01 ---
  engine.setUncategorizedIncome('2026-01', 500000); // $5,000
  engine.setBudget('2026-01', 'cat_rent', 150000);  // $1,500
  engine.setBudget('2026-01', 'cat_util', 20000);   // $200
  engine.setBudget('2026-01', 'cat_groc', 60000);   // $600
  engine.setBudget('2026-01', 'cat_dine', 15000);   // $150

  engine.setSpent('2026-01', 'cat_rent', -150000);  // -$1,500 -> leftover 0
  engine.setSpent('2026-01', 'cat_util', -25000);   // -$250 -> leftover -50 (uncarried)
  engine.setSpent('2026-01', 'cat_groc', -45000);   // -$450 -> leftover +150 (surplus)
  engine.setSpent('2026-01', 'cat_dine', -20000);   // -$200 -> leftover -50 (carried)
  engine.setCarryover('2026-01', 'cat_dine', true);

  const calcM1 = engine.calculateMonth('2026-01');

  // --- Month 2: 2026-02 ---
  engine.setUncategorizedIncome('2026-02', 450000); // $4,500
  engine.setBudget('2026-02', 'cat_rent', 150000);  // $1,500
  engine.setBudget('2026-02', 'cat_util', 22000);   // $220
  engine.setBudget('2026-02', 'cat_groc', 50000);   // $500
  engine.setBudget('2026-02', 'cat_dine', 20000);   // $200

  engine.setSpent('2026-02', 'cat_rent', -150000);
  engine.setSpent('2026-02', 'cat_util', -18000);
  engine.setSpent('2026-02', 'cat_groc', -52000);
  engine.setSpent('2026-02', 'cat_dine', -10000);

  const calcM2 = engine.calculateMonth('2026-02');

  // --- Month 3: 2026-03 ---
  engine.setUncategorizedIncome('2026-03', 480000); // $4,800
  engine.setBudget('2026-03', 'cat_rent', 150000);
  engine.setBudget('2026-03', 'cat_util', 20000);
  engine.setBudget('2026-03', 'cat_groc', 55000);
  engine.setBudget('2026-03', 'cat_dine', 25000);

  engine.setSpent('2026-03', 'cat_rent', -150000);
  engine.setSpent('2026-03', 'cat_util', -19500);
  engine.setSpent('2026-03', 'cat_groc', -48000);
  engine.setSpent('2026-03', 'cat_dine', -21000);

  const calcM3 = engine.calculateMonth('2026-03');

  const fixture = {
    scenario_name: "multi_month_rollover_scenario",
    description: "3-month progression testing positive rollover, uncarried overspending, and carried overspending identical to upstream Actual Budget",
    groups,
    categories,
    months: [
      {
        month: '2026-01',
        uncategorized_income: 500000,
        allocations: [
          { category_id: 'cat_rent', budgeted: 150000, spent: -150000, carryover: false },
          { category_id: 'cat_util', budgeted: 20000, spent: -25000, carryover: false },
          { category_id: 'cat_groc', budgeted: 60000, spent: -45000, carryover: false },
          { category_id: 'cat_dine', budgeted: 15000, spent: -20000, carryover: true },
        ],
        expected: calcM1
      },
      {
        month: '2026-02',
        uncategorized_income: 450000,
        allocations: [
          { category_id: 'cat_rent', budgeted: 150000, spent: -150000, carryover: false },
          { category_id: 'cat_util', budgeted: 22000, spent: -18000, carryover: false },
          { category_id: 'cat_groc', budgeted: 50000, spent: -52000, carryover: false },
          { category_id: 'cat_dine', budgeted: 20000, spent: -10000, carryover: false },
        ],
        expected: calcM2
      },
      {
        month: '2026-03',
        uncategorized_income: 480000,
        allocations: [
          { category_id: 'cat_rent', budgeted: 150000, spent: -150000, carryover: false },
          { category_id: 'cat_util', budgeted: 20000, spent: -19500, carryover: false },
          { category_id: 'cat_groc', budgeted: 55000, spent: -48000, carryover: false },
          { category_id: 'cat_dine', budgeted: 25000, spent: -21000, carryover: false },
        ],
        expected: calcM3
      }
    ]
  };

  const outPath = path.join(fixturesDir, 'multi_month_rollover_scenario.json');
  fs.writeFileSync(outPath, JSON.stringify(fixture, null, 2), 'utf-8');
  console.log(`Wrote ${outPath}`);
}

// Scenario 2: Mid-Stream Adjustment Cascade Scenario
function generateMidStreamAdjustment() {
  const engine = new ReferenceEnvelopeEngine();

  const groups = [
    { id: 'grp_main', name: 'Main Expenses', is_income: false },
  ];
  const categories = [
    { id: 'cat_savings', group_id: 'grp_main', name: 'Emergency Savings', is_income: false },
    { id: 'cat_leisure', group_id: 'grp_main', name: 'Leisure', is_income: false },
  ];

  groups.forEach(g => engine.registerGroup(g));
  categories.forEach(c => engine.registerCategory(c));

  // Initial 3 months
  engine.setUncategorizedIncome('2026-01', 300000);
  engine.setBudget('2026-01', 'cat_savings', 100000);
  engine.setBudget('2026-01', 'cat_leisure', 50000);

  engine.setUncategorizedIncome('2026-02', 300000);
  engine.setBudget('2026-02', 'cat_savings', 100000);
  engine.setBudget('2026-02', 'cat_leisure', 50000);

  engine.setUncategorizedIncome('2026-03', 300000);
  engine.setBudget('2026-03', 'cat_savings', 100000);
  engine.setBudget('2026-03', 'cat_leisure', 50000);

  const initialM1 = engine.calculateMonth('2026-01');
  const initialM2 = engine.calculateMonth('2026-02');
  const initialM3 = engine.calculateMonth('2026-03');

  // Midstream modification: Change Jan savings budget from $1,000 to $1,500
  engine.setBudget('2026-01', 'cat_savings', 150000);

  const updatedM1 = engine.calculateMonth('2026-01');
  const updatedM2 = engine.calculateMonth('2026-02');
  const updatedM3 = engine.calculateMonth('2026-03');

  const fixture = {
    scenario_name: "cascading_midstream_adjustment",
    description: "Tests mid-stream retro-active budget modifications in an earlier month cascading forward into future months",
    groups,
    categories,
    initial: {
      '2026-01': initialM1,
      '2026-02': initialM2,
      '2026-03': initialM3,
    },
    modification: {
      month: '2026-01',
      category_id: 'cat_savings',
      new_budget: 150000
    },
    after_modification: {
      '2026-01': updatedM1,
      '2026-02': updatedM2,
      '2026-03': updatedM3,
    }
  };

  const outPath = path.join(fixturesDir, 'cascading_midstream_adjustment.json');
  fs.writeFileSync(outPath, JSON.stringify(fixture, null, 2), 'utf-8');
  console.log(`Wrote ${outPath}`);
}

generateMultiMonthRollover();
generateMidStreamAdjustment();
