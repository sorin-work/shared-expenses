export interface Category {
	id: string;
	label: string;
	colour: string;
}

// Same list as the API (domain/category.rs).
export const CATEGORIES: Category[] = [
	{ id: 'general', label: 'General', colour: '#8a8f98' },
	{ id: 'groceries', label: 'Groceries', colour: '#3f9b5b' },
	{ id: 'rent', label: 'Rent', colour: '#7a5af5' },
	{ id: 'utilities', label: 'Utilities', colour: '#2b8fd6' },
	{ id: 'dining', label: 'Eating out', colour: '#e0703a' },
	{ id: 'transport', label: 'Transport', colour: '#c9a227' },
	{ id: 'travel', label: 'Travel', colour: '#1aa5a5' },
	{ id: 'accommodation', label: 'Accommodation', colour: '#b05fc2' },
	{ id: 'entertainment', label: 'Entertainment', colour: '#d6457a' },
	{ id: 'household', label: 'Household', colour: '#6b8e23' },
	{ id: 'gifts', label: 'Gifts', colour: '#e05555' },
	{ id: 'health', label: 'Health', colour: '#3aa0a0' }
];

export function category(id: string): Category {
	return CATEGORIES.find((c) => c.id === id) ?? CATEGORIES[0];
}
