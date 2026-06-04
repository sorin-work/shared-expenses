/** Today's date in the local timezone as YYYY-MM-DD. */
export function today(now: Date = new Date()): string {
	const y = now.getFullYear();
	const m = String(now.getMonth() + 1).padStart(2, '0');
	const d = String(now.getDate()).padStart(2, '0');
	return `${y}-${m}-${d}`;
}

/** "12 Mar 2026" for a YYYY-MM-DD date, without timezone shifts. */
export function formatDate(isoDate: string, locale?: string): string {
	const [y, m, d] = isoDate.slice(0, 10).split('-').map(Number);
	return new Intl.DateTimeFormat(locale, {
		day: 'numeric',
		month: 'short',
		year: 'numeric',
		timeZone: 'UTC'
	}).format(new Date(Date.UTC(y, m - 1, d)));
}

/** "just now", "5 min ago", "3 h ago", "yesterday", "4 days ago" or a date. */
export function timeAgo(iso: string, now: Date = new Date(), locale?: string): string {
	const seconds = Math.round((now.getTime() - new Date(iso).getTime()) / 1000);
	if (seconds < 60) return 'just now';
	const minutes = Math.floor(seconds / 60);
	if (minutes < 60) return `${minutes} min ago`;
	const hours = Math.floor(minutes / 60);
	if (hours < 24) return `${hours} h ago`;
	const days = Math.floor(hours / 24);
	if (days === 1) return 'yesterday';
	if (days < 7) return `${days} days ago`;
	return formatDate(new Date(iso).toISOString(), locale);
}

/** Groups items by a YYYY-MM month key, keeping their order. */
export function groupByMonth<T>(items: T[], dateOf: (item: T) => string): [string, T[]][] {
	const groups = new Map<string, T[]>();
	for (const item of items) {
		const key = dateOf(item).slice(0, 7);
		const list = groups.get(key);
		if (list) list.push(item);
		else groups.set(key, [item]);
	}
	return [...groups.entries()];
}

export function formatMonth(key: string, locale?: string): string {
	const [y, m] = key.split('-').map(Number);
	return new Intl.DateTimeFormat(locale, {
		month: 'long',
		year: 'numeric',
		timeZone: 'UTC'
	}).format(new Date(Date.UTC(y, m - 1, 1)));
}
