//! WebDAV property definitions and XML serialization (RFC 4918).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// DAV namespace
pub const DAV_NS: &str = "DAV:";

/// A WebDAV resource (file or collection)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DavResource {
    /// Full href path for this resource
    pub href: String,
    /// Whether this is a collection (directory) or a file
    pub is_collection: bool,
    /// Display name
    pub display_name: String,
    /// Content length in bytes (0 for collections)
    pub content_length: u64,
    /// Content type (MIME)
    pub content_type: String,
    /// Last modified timestamp
    pub last_modified: DateTime<Utc>,
    /// Creation date
    pub creation_date: DateTime<Utc>,
    /// ETag for cache validation
    pub etag: Option<String>,
}

impl DavResource {
    /// Create a collection (folder) resource
    pub fn collection(
        href: String,
        name: String,
        modified: DateTime<Utc>,
        created: DateTime<Utc>,
    ) -> Self {
        Self {
            href,
            is_collection: true,
            display_name: name,
            content_length: 0,
            content_type: "httpd/unix-directory".to_string(),
            last_modified: modified,
            creation_date: created,
            etag: None,
        }
    }

    /// Create a file resource
    pub fn file(
        href: String,
        name: String,
        size: u64,
        mime: String,
        modified: DateTime<Utc>,
        created: DateTime<Utc>,
        etag: Option<String>,
    ) -> Self {
        Self {
            href,
            is_collection: false,
            display_name: name,
            content_length: size,
            content_type: mime,
            last_modified: modified,
            creation_date: created,
            etag,
        }
    }
}

/// Format a DateTime as HTTP date (RFC 7231)
pub fn format_http_date(dt: &DateTime<Utc>) -> String {
    dt.format("%a, %d %b %Y %H:%M:%S GMT").to_string()
}

/// Format a DateTime as ISO 8601 for WebDAV creationdate
pub fn format_creation_date(dt: &DateTime<Utc>) -> String {
    dt.to_rfc3339()
}

/// Depth header values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Depth {
    /// Only the resource itself
    Zero,
    /// Resource and its immediate children
    One,
    /// Resource and all descendants
    Infinity,
}

impl Depth {
    /// Parse from a header value string
    pub fn from_header(value: Option<&str>) -> Self {
        match value {
            Some("0") => Self::Zero,
            Some("1") => Self::One,
            Some("infinity") | Some("Infinity") => Self::Infinity,
            None => Self::Infinity,
            _ => Self::Infinity,
        }
    }
}

/// Generate a multistatus XML response
pub fn build_multistatus_xml(resources: &[DavResource]) -> String {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
    xml.push_str("<D:multistatus xmlns:D=\"DAV:\">\n");

    for resource in resources {
        xml.push_str("  <D:response>\n");
        xml.push_str(&format!(
            "    <D:href>{}</D:href>\n",
            xml_escape(&resource.href)
        ));
        xml.push_str("    <D:propstat>\n");
        xml.push_str("      <D:prop>\n");

        // displayname
        xml.push_str(&format!(
            "        <D:displayname>{}</D:displayname>\n",
            xml_escape(&resource.display_name)
        ));

        // resourcetype
        if resource.is_collection {
            xml.push_str("        <D:resourcetype><D:collection/></D:resourcetype>\n");
        } else {
            xml.push_str("        <D:resourcetype/>\n");
        }

        // getcontentlength
        if !resource.is_collection {
            xml.push_str(&format!(
                "        <D:getcontentlength>{}</D:getcontentlength>\n",
                resource.content_length
            ));
        }

        // getcontenttype
        xml.push_str(&format!(
            "        <D:getcontenttype>{}</D:getcontenttype>\n",
            xml_escape(&resource.content_type)
        ));

        // getlastmodified
        xml.push_str(&format!(
            "        <D:getlastmodified>{}</D:getlastmodified>\n",
            format_http_date(&resource.last_modified)
        ));

        // creationdate
        xml.push_str(&format!(
            "        <D:creationdate>{}</D:creationdate>\n",
            format_creation_date(&resource.creation_date)
        ));

        // getetag
        if let Some(ref etag) = resource.etag {
            xml.push_str(&format!(
                "        <D:getetag>\"{}\"</D:getetag>\n",
                xml_escape(etag)
            ));
        }

        xml.push_str("      </D:prop>\n");
        xml.push_str("      <D:status>HTTP/1.1 200 OK</D:status>\n");
        xml.push_str("    </D:propstat>\n");
        xml.push_str("  </D:response>\n");
    }

    xml.push_str("</D:multistatus>\n");
    xml
}

/// Build a simple status XML response (e.g. for PROPPATCH)
pub fn build_status_xml(href: &str, status_code: u16, description: &str) -> String {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
    xml.push_str("<D:multistatus xmlns:D=\"DAV:\">\n");
    xml.push_str("  <D:response>\n");
    xml.push_str(&format!("    <D:href>{}</D:href>\n", xml_escape(href)));
    xml.push_str(&format!(
        "    <D:status>HTTP/1.1 {} {}</D:status>\n",
        status_code, description
    ));
    xml.push_str("  </D:response>\n");
    xml.push_str("</D:multistatus>\n");
    xml
}

/// Build an error XML response
pub fn build_error_xml(error_tag: &str, description: &str) -> String {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
    xml.push_str("<D:error xmlns:D=\"DAV:\">\n");
    xml.push_str(&format!("  <D:{}/>\n", xml_escape(error_tag)));
    xml.push_str(&format!(
        "  <D:responsedescription>{}</D:responsedescription>\n",
        xml_escape(description)
    ));
    xml.push_str("</D:error>\n");
    xml
}

/// Escape XML special characters
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Parse a PROPFIND request body to determine requested properties
#[derive(Debug, Clone)]
pub enum PropfindRequest {
    /// Request all properties
    AllProp,
    /// Request specific properties by name
    Prop(Vec<String>),
    /// Request property names only
    PropName,
}

impl PropfindRequest {
    /// Parse from XML body (simplified parser)
    pub fn parse(body: &str) -> Self {
        if body.is_empty() {
            return Self::AllProp;
        }

        let lower = body.to_lowercase();

        if lower.contains("<d:allprop") || lower.contains("<allprop") {
            Self::AllProp
        } else if lower.contains("<d:propname") || lower.contains("<propname") {
            Self::PropName
        } else {
            Self::AllProp
        }
    }
}
