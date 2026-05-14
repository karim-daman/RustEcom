const userInfo = document.getElementById('user-info');
const createUserButton = document.getElementById('create-user');
const tableSelect = document.getElementById('table-select');
const operationSelect = document.getElementById('operation-select');
const idLabel = document.getElementById('id-label');
const rowIdInput = document.getElementById('row-id');
const bodyLabel = document.getElementById('body-label');
const requestBody = document.getElementById('request-body');
const fillSampleButton = document.getElementById('fill-sample');
const executeRequestButton = document.getElementById('execute-request');
const requestDisplay = document.getElementById('request-display');
const responseDisplay = document.getElementById('response-display');
const metadataContainer = document.getElementById('metadata');

let currentUserId = localStorage.getItem('rustecom_user_id');
let tableMetadata = [];

function api(path, options = {}) {
  const headers = options.body ? { 'Content-Type': 'application/json' } : {};
  return fetch(path, { headers, ...options }).then(async (res) => {
    const text = await res.text();
    if (!res.ok) {
      let errorMessage = 'API error';
      try {
        const body = JSON.parse(text);
        errorMessage = body.error || JSON.stringify(body);
      } catch (e) {
        errorMessage = text || errorMessage;
      }
      throw new Error(errorMessage);
    }
    if (!text) return null;
    try {
      return JSON.parse(text);
    } catch (e) {
      return text;
    }
  });
}

function setUserInfo(text) {
  userInfo.textContent = text;
}

function displayRequest(request) {
  requestDisplay.textContent = request;
}

function displayResponse(response) {
  responseDisplay.textContent = typeof response === 'string' ? response : JSON.stringify(response, null, 2);
}

function updateFormState() {
  const operation = operationSelect.value;
  const needsId = operation === 'get' || operation === 'update' || operation === 'delete';
  const needsBody = operation === 'create' || operation === 'update';

  idLabel.classList.toggle('hidden', !needsId);
  bodyLabel.classList.toggle('hidden', !needsBody);

  if (!needsId) {
    rowIdInput.value = '';
  }
  if (!needsBody) {
    requestBody.value = '';
  }
  updateRequestPreview();
}

function updateRequestPreview() {
  const table = tableSelect.value;
  const operation = operationSelect.value;
  const id = rowIdInput.value.trim();
  const body = requestBody.value.trim();
  let method = 'GET';
  let path = `/api/table/${table}`;

  if (operation === 'create') {
    method = 'POST';
  } else if (operation === 'get') {
    method = 'GET';
    path += `/${encodeURIComponent(id)}`;
  } else if (operation === 'update') {
    method = 'PUT';
    path += `/${encodeURIComponent(id)}`;
  } else if (operation === 'delete') {
    method = 'DELETE';
    path += `/${encodeURIComponent(id)}`;
  }

  displayRequest(`${method} ${path}${body ? '\n\n' + body : ''}`);
}

function buildSampleBody() {
  const table = tableSelect.value;
  const meta = tableMetadata.find((item) => item.name === table);
  if (!meta) {
    requestBody.value = '';
    return;
  }

  const sample = {};
  meta.columns.forEach((column) => {
    if (meta.primary_keys.includes(column) && column === 'id') {
      return;
    }
    if (column.endsWith('_id')) {
      sample[column] = '00000000-0000-0000-0000-000000000000';
    } else if (column.includes('price') || column.includes('amount') || column.includes('quantity')) {
      sample[column] = 100;
    } else if (column === 'is_active' || column === 'is_default') {
      sample[column] = true;
    } else if (column === 'status') {
      sample[column] = 'pending';
    } else {
      sample[column] = `${column}-value`;
    }
  });

  requestBody.value = JSON.stringify(sample, null, 2);
  updateRequestPreview();
}

async function loadMetadata() {
  try {
    const metadata = await api('/api/table/metadata');
    tableMetadata = metadata;
    tableSelect.innerHTML = metadata.map((table) => `<option value="${table.name}">${table.name}</option>`).join('');

    metadataContainer.innerHTML = metadata
      .map(
        (table) => `
          <div class="meta-card">
            <h3>${table.name}</h3>
            <p><strong>Columns:</strong> ${table.columns.join(', ')}</p>
            <p><strong>Primary keys:</strong> ${table.primary_keys.join(', ')}</p>
          </div>
        `
      )
      .join('');

    updateRequestPreview();
  } catch (error) {
    displayResponse(error.message);
  }
}

async function createDemoUser() {
  try {
    const user = await api('/api/users/demo', { method: 'POST' });
    currentUserId = user.id;
    localStorage.setItem('rustecom_user_id', user.id);
    setUserInfo(`Demo user: ${user.email}`);
    displayResponse(user);
  } catch (error) {
    displayResponse(error.message);
  }
}

async function executeRequest() {
  const table = tableSelect.value;
  const operation = operationSelect.value;
  const id = rowIdInput.value.trim();
  let path = `/api/table/${table}`;
  let method = 'GET';
  const options = {};

  if (operation === 'create') {
    method = 'POST';
  } else if (operation === 'get') {
    method = 'GET';
    path += `/${encodeURIComponent(id)}`;
  } else if (operation === 'update') {
    method = 'PUT';
    path += `/${encodeURIComponent(id)}`;
  } else if (operation === 'delete') {
    method = 'DELETE';
    path += `/${encodeURIComponent(id)}`;
  }

  if (operation === 'create' || operation === 'update') {
    try {
      options.body = JSON.stringify(JSON.parse(requestBody.value));
    } catch (error) {
      displayResponse('Invalid JSON body: ' + error.message);
      return;
    }
  }

  options.method = method;
  try {
    const result = await api(path, options);
    displayResponse(result);
  } catch (error) {
    displayResponse(error.message);
  }
}

createUserButton.addEventListener('click', createDemoUser);
operationSelect.addEventListener('change', updateFormState);
rowIdInput.addEventListener('input', updateRequestPreview);
requestBody.addEventListener('input', updateRequestPreview);
fillSampleButton.addEventListener('click', buildSampleBody);
executeRequestButton.addEventListener('click', executeRequest);

document.addEventListener('DOMContentLoaded', () => {
  if (currentUserId) {
    setUserInfo(`Stored demo user ID: ${currentUserId}`);
  }
  updateFormState();
  loadMetadata();
});
