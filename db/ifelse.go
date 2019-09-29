// Copyright © 2016 Abcum Ltd
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

package db

import (
	"context"

	"github.com/abcum/surreal/sql"
	"github.com/abcum/surreal/util/data"
)

func (e *executor) executeIfelse(ctx context.Context, stm *sql.IfelseStatement) (out []interface{}, err error) {

	val, err := e.fetchIfelse(ctx, stm, nil)
	if err != nil {
		return nil, err
	}

	switch val := val.(type) {
	case []interface{}:
		out = val
	case interface{}:
		out = append(out, val)
	}

	return

}

func (e *executor) fetchIfelse(ctx context.Context, stm *sql.IfelseStatement, doc *data.Doc) (interface{}, error) {

	for k, v := range stm.Cond {
		ife, err := e.fetch(ctx, v, doc)
		if err != nil {
			return nil, err
		}
		if calcAsBool(ife) {
			return e.fetch(ctx, stm.Then[k], doc)
		}
	}

	return e.fetch(ctx, stm.Else, doc)

}