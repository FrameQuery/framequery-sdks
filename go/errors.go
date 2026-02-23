package framequery

import "fmt"

// Error is an API error. StatusCode is 0 for non-HTTP errors (e.g. job failure).
type Error struct {
	Message    string
	StatusCode int
	Body       map[string]any
}

func (e *Error) Error() string {
	if e.StatusCode > 0 {
		return fmt.Sprintf("framequery: API error %d: %s", e.StatusCode, e.Message)
	}
	return fmt.Sprintf("framequery: %s", e.Message)
}

// IsAuthError checks for 401 Unauthorized.
func IsAuthError(err error) bool {
	e, ok := err.(*Error)
	return ok && e.StatusCode == 401
}

// IsNotFoundError checks for 404 Not Found.
func IsNotFoundError(err error) bool {
	e, ok := err.(*Error)
	return ok && e.StatusCode == 404
}

// IsRateLimitError checks for 429 Too Many Requests.
func IsRateLimitError(err error) bool {
	e, ok := err.(*Error)
	return ok && e.StatusCode == 429
}

// IsPermissionError checks for 403 Forbidden.
func IsPermissionError(err error) bool {
	e, ok := err.(*Error)
	return ok && e.StatusCode == 403
}
