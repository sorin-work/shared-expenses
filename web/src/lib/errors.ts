import { ApiError } from './api';

/** Message to show for any error thrown by an API call. */
export function errorMessage(err: unknown): string {
	if (err instanceof ApiError) {
		if (err.code === 'validation_failed' && Object.keys(err.fields).length) {
			return 'Please fix the highlighted fields.';
		}
		return err.message.charAt(0).toUpperCase() + err.message.slice(1);
	}
	return 'Something went wrong. Please try again.';
}

/** Field errors from a validation failure, with capitalised messages. */
export function fieldErrors(err: unknown): Record<string, string> {
	if (!(err instanceof ApiError)) return {};
	return Object.fromEntries(
		Object.entries(err.fields).map(([k, v]) => [k, v.charAt(0).toUpperCase() + v.slice(1)])
	);
}
