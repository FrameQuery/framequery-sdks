package framequery

import "fmt"

// Error represents an API error from FrameQuery.
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

// IsAuthError returns true if the error is an authentication failure (401).
func IsAuthError(err error) bool {
	e, ok := err.(*Error)
	return ok && e.StatusCode == 401
}

// IsNotFoundError returns true if the error is a not-found response (404).
func IsNotFoundError(err error) bool {
	e, ok := err.(*Error)
	return ok && e.StatusCode == 404
}

// IsRateLimitError returns true if the error is a rate limit response (429).
func IsRateLimitError(err error) bool {
	e, ok := err.(*Error)
	return ok && e.StatusCode == 429
}

// IsPermissionError returns true if the error is a permission denied response (403).
func IsPermissionError(err error) bool {
	e, ok := err.(*Error)
	return ok && e.StatusCode == 403
}
