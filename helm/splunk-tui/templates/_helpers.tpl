{{/*
Expand the name of the chart.
*/}}
{{- define "splunk-tui.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
We truncate at 63 chars because some Kubernetes name fields are limited to this (by the DNS naming spec).
If release name contains chart name it will be used as a full name.
*/}}
{{- define "splunk-tui.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "splunk-tui.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "splunk-tui.labels" -}}
helm.sh/chart: {{ include "splunk-tui.chart" . }}
{{ include "splunk-tui.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "splunk-tui.selectorLabels" -}}
app.kubernetes.io/name: {{ include "splunk-tui.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Create the name of the service account to use
*/}}
{{- define "splunk-tui.serviceAccountName" -}}
{{- if .Values.serviceAccount.create }}
{{- default (include "splunk-tui.fullname" .) .Values.serviceAccount.name }}
{{- else }}
{{- default "default" .Values.serviceAccount.name }}
{{- end }}
{{- end }}

{{/*
Get the secret name for CLI
*/}}
{{- define "splunk-tui.cliSecretName" -}}
{{- if .Values.cli.existingSecret }}
{{- .Values.cli.existingSecret }}
{{- else }}
{{- printf "%s-cli" (include "splunk-tui.fullname" .) }}
{{- end }}
{{- end }}

{{/*
Get the secret name for TUI
*/}}
{{- define "splunk-tui.tuiSecretName" -}}
{{- if .Values.tui.existingSecret }}
{{- .Values.tui.existingSecret }}
{{- else }}
{{- printf "%s-tui" (include "splunk-tui.fullname" .) }}
{{- end }}
{{- end }}
