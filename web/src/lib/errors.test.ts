import { describe, expect, it } from 'vitest';
import { ApiError } from './api';
import { errorMessage } from './errors';

describe('errorMessage', () => {
	it('capitalises api messages', () => {
		expect(errorMessage(new ApiError(404, 'not_found', 'group not found'))).toBe('Group not found');
	});

	it('points at the fields for validation errors', () => {
		const err = new ApiError(422, 'validation_failed', 'some fields are invalid', { name: 'x' });
		expect(errorMessage(err)).toBe('Please fix the highlighted fields.');
	});

	it('has a generic message for anything else', () => {
		expect(errorMessage(new Error('boom'))).toBe('Something went wrong. Please try again.');
	});
});
