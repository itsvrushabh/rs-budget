/**
 * Node.js reference calculation implementation matching upstream Actual Budget (`v26.11.0` loot-core).
 * Used to produce deterministic golden test vectors for rs-budget differential testing.
 */

function prevMonth(monthStr) {
  const [yearStr, monthNumStr] = monthStr.split('-');
  const year = parseInt(yearStr, 10);
  const month = parseInt(monthNumStr, 10);
  if (month === 1) {
    return `${year - 1}-12`;
  }
  return `${year}-${String(month - 1).padStart(2, '0')}`;
}

function nextMonth(monthStr) {
  const [yearStr, monthNumStr] = monthStr.split('-');
  const year = parseInt(yearStr, 10);
  const month = parseInt(monthNumStr, 10);
  if (month === 12) {
    return `${year + 1}-01`;
  }
  return `${year}-${String(month + 1).padStart(2, '0')}`;
}

class ReferenceEnvelopeEngine {
  constructor() {
    this.months = new Map(); // month -> { categories, groups, budgeted, spent, carryover, buffered, uncat_income }
    this.categories = new Map();
    this.groups = new Map();
  }

  registerCategory(cat) {
    this.categories.set(cat.id, { ...cat });
  }

  registerGroup(grp) {
    this.groups.set(grp.id, { ...grp });
  }

  initMonth(month) {
    if (!this.months.has(month)) {
      this.months.set(month, {
        budgeted: new Map(),
        spent: new Map(),
        carryover: new Map(),
        buffered: 0,
        uncategorized_income: 0,
      });
      for (const cat of this.categories.values()) {
        this.months.get(month).budgeted.set(cat.id, 0);
        this.months.get(month).spent.set(cat.id, 0);
        this.months.get(month).carryover.set(cat.id, false);
      }
    }
  }

  setBudget(month, catId, amount) {
    this.initMonth(month);
    this.months.get(month).budgeted.set(catId, amount);
  }

  setSpent(month, catId, amount) {
    this.initMonth(month);
    this.months.get(month).spent.set(catId, amount);
  }

  setCarryover(month, catId, carryover) {
    this.initMonth(month);
    this.months.get(month).carryover.set(catId, !!carryover);
  }

  setUncategorizedIncome(month, amount) {
    this.initMonth(month);
    this.months.get(month).uncategorized_income = amount;
  }

  setBuffered(month, amount) {
    this.initMonth(month);
    this.months.get(month).buffered = amount;
  }

  addTransaction(tx) {
    const month = tx.date.substring(0, 7);
    this.initMonth(month);
    if (tx.category_id) {
      const cur = this.months.get(month).spent.get(tx.category_id) || 0;
      this.months.get(month).spent.set(tx.category_id, cur + tx.amount);
    } else if (tx.amount > 0) {
      this.months.get(month).uncategorized_income += tx.amount;
    }
  }

  calculateMonth(month) {
    const prevM = prevMonth(month);
    const hasPrev = this.months.has(prevM);
    const mData = this.months.get(month);

    let prevCalculations = null;
    if (hasPrev) {
      prevCalculations = this.calculateMonth(prevM);
    }

    const categoryBalances = {};
    let totalBudgeted = 0;
    let totalSpent = 0;
    let totalLeftover = 0;
    let totalIncome = mData.uncategorized_income;
    let lastMonthOverspent = 0;

    // Calculate category leftovers
    for (const cat of this.categories.values()) {
      const budgeted = mData.budgeted.get(cat.id) || 0;
      const spent = mData.spent.get(cat.id) || 0;
      const carryover = mData.carryover.get(cat.id) || false;

      let leftover = budgeted + spent;

      if (!cat.is_income) {
        totalBudgeted += budgeted;
        totalSpent += spent;

        if (hasPrev) {
          const prevLeftover = prevCalculations.category_balances[cat.id]?.leftover || 0;
          const prevCarry = this.months.get(prevM).carryover.get(cat.id) || false;
          const carry = prevCarry ? prevLeftover : Math.max(0, prevLeftover);
          leftover += carry;

          if (!prevCarry && prevLeftover < 0) {
            lastMonthOverspent += prevLeftover;
          }
        }
        totalLeftover += leftover;
      } else {
        totalIncome += spent;
      }

      categoryBalances[cat.id] = {
        budgeted,
        spent,
        leftover,
        carryover,
      };
    }

    const fromLastMonth = hasPrev ? prevCalculations.to_budget + this.months.get(prevM).buffered : 0;
    const availableFunds = totalIncome + fromLastMonth;
    const toBudget = availableFunds + lastMonthOverspent - totalBudgeted - mData.buffered;

    // Group calculations
    const groupBalances = {};
    for (const grp of this.groups.values()) {
      let gBudgeted = 0;
      let gSpent = 0;
      let gLeftover = 0;

      for (const cat of this.categories.values()) {
        if (cat.group_id === grp.id) {
          gBudgeted += categoryBalances[cat.id].budgeted;
          gSpent += categoryBalances[cat.id].spent;
          gLeftover += categoryBalances[cat.id].leftover;
        }
      }

      groupBalances[grp.id] = {
        budgeted: gBudgeted,
        spent: gSpent,
        leftover: gLeftover,
      };
    }

    return {
      month,
      total_income: totalIncome,
      from_last_month: fromLastMonth,
      available_funds: availableFunds,
      last_month_overspent: lastMonthOverspent,
      total_budgeted: totalBudgeted,
      total_spent: totalSpent,
      total_leftover: totalLeftover,
      to_budget: toBudget,
      category_balances: categoryBalances,
      group_balances: groupBalances,
    };
  }
}

module.exports = {
  ReferenceEnvelopeEngine,
  prevMonth,
  nextMonth,
};
