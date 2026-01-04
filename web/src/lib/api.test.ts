import { describe, expect, it, vi } from 'vitest';
import { ApiError, request } from './api';

function fakeFetch(status: number, body?: unknown, contentType = 'application/json') {
	return vi.fn(
		async () =>
			new Response(body === undefined ? null : JSON.stringify(body), {
				status,
				headers: { 'Content-Type': contentType }
			})
	);
}

/** The ApiError a failing request rejects with. */
async function failure(promise: Promise<unknown>): Promise<ApiError> {
	try {
		await promise;
	} catch (err) {
		if (err instanceof ApiError) return err;
		throw err;
	}
	throw new Error('expected the request to fail');
}

describe('request', () => {
	it('sends json with the token and parses the response', async () => {
		const fetch = fakeFetch(200, { id: 'g1' });
		const result = await request<{ id: string }>('/api/groups', {
			method: 'POST',
			body: { name: 'Flat' },
			token: 'abc',
			fetch
		});
		expect(result).toEqual({ id: 'g1' });
		const [path, init] = fetch.mock.calls[0] as unknown as [string, RequestInit];
		expect(path).toBe('/api/groups');
		expect(init.method).toBe('POST');
		expect(init.body).toBe('{"name":"Flat"}');
		expect((init.headers as Record<string, string>).Authorization).toBe('Bearer abc');
	});

	it('returns undefined for 204 responses', async () => {
		const result = await request('/api/x', { method: 'DELETE', fetch: fakeFetch(204) });
		expect(result).toBeUndefined();
	});

	it('turns api errors into ApiError with fields', async () => {
		const fetch = fakeFetch(422, {
			error: {
				code: 'validation_failed',
				message: 'some fields are invalid',
				fields: { name: 'is required' }
			}
		});
		const err = await failure(request('/api/groups', { fetch }));
		expect(err).toBeInstanceOf(ApiError);
		expect(err.status).toBe(422);
		expect(err.code).toBe('validation_failed');
		expect(err.field('name')).toBe('is required');
	});

	it('copes with non-json error pages', async () => {
		const fetch = vi.fn(async () => new Response('<html>Bad gateway</html>', { status: 502 }));
		const err = await failure(request('/api/groups', { fetch }));
		expect(err.code).toBe('http_error');
		expect(err.message).toMatch(/server had a problem/);
	});

	it('reports network failures', async () => {
		const fetch = vi.fn(async () => {
			throw new TypeError('Failed to fetch');
		});
		const err = await failure(request('/api/groups', { fetch }));
		expect(err.status).toBe(0);
		expect(err.code).toBe('network_error');
	});
});
