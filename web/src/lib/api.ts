// Thin wrapper around fetch for the JSON API.

export class ApiError extends Error {
	constructor(
		readonly status: number,
		readonly code: string,
		message: string,
		readonly fields: Record<string, string> = {}
	) {
		super(message);
		this.name = 'ApiError';
	}

	/** The message for one form field, if the API flagged it. */
	field(name: string): string | undefined {
		return this.fields[name];
	}
}

export interface ApiOptions {
	method?: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';
	body?: unknown;
	token?: string | null;
	fetch?: typeof fetch;
	/** Called on a 401 for an authenticated request (expired session). */
	onUnauthorized?: () => void;
}

async function errorFrom(response: Response): Promise<ApiError> {
	try {
		const body = await response.json();
		const err = body?.error;
		if (err && typeof err.message === 'string') {
			return new ApiError(response.status, err.code ?? 'error', err.message, err.fields ?? {});
		}
	} catch {
		// Not JSON, e.g. a proxy error page.
	}
	return new ApiError(
		response.status,
		'http_error',
		response.status >= 500
			? 'The server had a problem. Please try again.'
			: `Request failed (${response.status})`
	);
}

export async function rawRequest(path: string, options: ApiOptions = {}): Promise<Response> {
	const headers: Record<string, string> = { Accept: 'application/json' };
	if (options.body !== undefined) headers['Content-Type'] = 'application/json';
	if (options.token) headers.Authorization = `Bearer ${options.token}`;

	const doFetch = options.fetch ?? fetch;
	let response: Response;
	try {
		response = await doFetch(path, {
			method: options.method ?? 'GET',
			headers,
			body: options.body === undefined ? undefined : JSON.stringify(options.body)
		});
	} catch {
		throw new ApiError(0, 'network_error', "Can't reach the server. Check your connection.");
	}

	if (!response.ok) {
		if (response.status === 401 && options.token) options.onUnauthorized?.();
		throw await errorFrom(response);
	}
	return response;
}

export async function request<T>(path: string, options: ApiOptions = {}): Promise<T> {
	const response = await rawRequest(path, options);
	if (response.status === 204) return undefined as T;
	return (await response.json()) as T;
}

/** Builds a query string, skipping empty values. */
export function query(params: Record<string, string | number | null | undefined>): string {
	const search = new URLSearchParams();
	for (const [key, value] of Object.entries(params)) {
		if (value !== null && value !== undefined && value !== '') search.set(key, String(value));
	}
	const s = search.toString();
	return s ? `?${s}` : '';
}
